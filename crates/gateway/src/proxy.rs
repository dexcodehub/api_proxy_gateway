use async_trait::async_trait;
use arc_swap::ArcSwap;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_core::Result;
use pingora_http::RequestHeader;
use pingora_load_balancing::selection::RoundRobin;
use pingora_load_balancing::LoadBalancer;
use pingora_proxy::{ProxyHttp, Session};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use serde_json::json;

use crate::circuit_breaker::CircuitBreaker;
use crate::config::ProxyConfig;
use crate::observability::{
    CIRCUIT_BREAKER_OPEN_TOTAL, REQUESTS_TOTAL, REQUEST_DURATION, RETRIES_TOTAL,
    UPSTREAM_ERRORS_TOTAL, UPSTREAM_SELECTED_TOTAL,
};
use crate::rate_limiter::RateLimiter;
use crate::retry::{retry_with_policy, RetryPolicy, RetryableError};

pub struct LB {
    pub load_balancer: Arc<LoadBalancer<RoundRobin>>,
    pub rate_limiter: RateLimiter,
    pub circuit_breaker: CircuitBreaker,
    pub retry_policy: RetryPolicy,
    pub config: Arc<ArcSwap<ProxyConfig>>,
}

#[derive(Clone, Debug)]
pub struct RequestCtx {
    pub start: std::time::Instant,
    pub request_id: Uuid,
    pub upstream_addr: Option<String>,
}

fn summarize_query(uri: &str) -> Vec<String> {
    if let Some(pos) = uri.find('?') {
        let q = &uri[pos + 1..];
        q.split('&')
            .filter_map(|pair| {
                if pair.is_empty() { return None; }
                let key = pair.split('=').next().unwrap_or("");
                if key.is_empty() { None } else { Some(key.to_string()) }
            })
            .collect()
    } else {
        Vec::new()
    }
}

#[async_trait]
impl ProxyHttp for LB {
    type CTX = RequestCtx;

    fn new_ctx(&self) -> Self::CTX {
        REQUESTS_TOTAL.inc();
        RequestCtx { start: std::time::Instant::now(), request_id: Uuid::new_v4(), upstream_addr: None }
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        // 请求入口日志（结构化、脱敏）
        let method = session.req_header().method.to_string();
        let uri = session.req_header().uri.to_string();
        let query_keys = summarize_query(&uri);
        info!(
            event = "request_start",
            request_id = %ctx.request_id,
            method = %method,
            uri = %uri,
            query_keys = ?query_keys,
            "incoming request"
        );
        // Check rate limiting
        if !self.rate_limiter.check_rate_limit().await {
            crate::observability::RATE_LIMITED_TOTAL.inc();
            warn!(event = "rate_limited", request_id = %ctx.request_id, reason = "rate limiter", "Request rejected by rate limiter");
            let _ = session.respond_error(429).await;
            return Ok(true);
        }
        debug!(event = "rate_limit_pass", request_id = %ctx.request_id, "rate limiter allowed request");

        // Check circuit breaker
        if !self.circuit_breaker.can_execute().await {
            CIRCUIT_BREAKER_OPEN_TOTAL.inc();
            warn!(event = "circuit_open", request_id = %ctx.request_id, reason = "circuit breaker", "Request rejected by circuit breaker");
            let _ = session.respond_error(503).await;
            return Ok(true);
        }
        debug!(event = "circuit_ok", request_id = %ctx.request_id, "circuit breaker allows execution");

        Ok(false)
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        debug!(event = "upstream_select_start", request_id = %ctx.request_id, "selecting upstream peer");
        let select_upstream = || async {
            match self.load_balancer.select(b"", 256) {
                Some(upstream) => {
                    UPSTREAM_SELECTED_TOTAL.inc();
                    debug!(event = "upstream_selected", peer = %format!("{:?}", upstream), "upstream peer selected");
                    let addr_str = format!("{}", upstream.addr);
                    let peer = Box::new(HttpPeer::new(upstream, false, String::new()));
                    Ok::<(Box<HttpPeer>, String), RetryableError>((peer, addr_str))
                }
                None => {
                    UPSTREAM_ERRORS_TOTAL.inc();
                    Err(RetryableError::retryable("no upstream available".to_string()))
                }
            }
        };

        match retry_with_policy(&self.retry_policy, select_upstream).await {
            Ok((peer, addr)) => {
                self.circuit_breaker.record_success().await;
                ctx.upstream_addr = Some(addr.clone());
                info!(event = "forward_start", request_id = %ctx.request_id, upstream = %addr, "forwarding request to upstream");
                debug!(event = "upstream_select_end", request_id = %ctx.request_id, "upstream selection succeeded");
                Ok(peer)
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                RETRIES_TOTAL.inc();
                error!(event = "upstream_select_failed", request_id = %ctx.request_id, error = %e, "Failed to select upstream after retries");
                Err(pingora_core::Error::new_str("upstream selection failed"))
            }
        }
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let config = self.config.load();
        if let Some(first_upstream) = config.upstreams.first() {
            upstream_request.insert_header("Host", first_upstream).unwrap();
        } else {
            upstream_request.insert_header("Host", "127.0.0.1:8080").unwrap();
        }
        // 传播请求ID到上游，便于链路追踪
        upstream_request.insert_header("X-Request-Id", &ctx.request_id.to_string()).ok();
        debug!(event = "header_injected", request_id = %ctx.request_id, upstream = %ctx.upstream_addr.as_deref().unwrap_or(""), "injected Host and X-Request-Id headers to upstream request");
        Ok(())
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut pingora_http::ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let duration = ctx.start.elapsed();
        REQUEST_DURATION.observe(duration.as_secs_f64());
        info!(
            event = "response_headers",
            request_id = %ctx.request_id,
            upstream = %ctx.upstream_addr.as_deref().unwrap_or(""),
            status = %format!("{:?}", upstream_response.status),
            "upstream response received"
        );
        Ok(())
    }

    async fn logging(
        &self,
        session: &mut Session,
        e: Option<&pingora_core::Error>,
        ctx: &mut Self::CTX,
    ) {
        let duration = ctx.start.elapsed();
        let method = session.req_header().method.to_string();
        let uri = session.req_header().uri.to_string();

        if let Some(err) = e {
            error!(
                event = "request_error",
                request_id = %ctx.request_id,
                method = %method,
                uri = %uri,
                duration_ms = %duration.as_millis(),
                upstream = %ctx.upstream_addr.as_deref().unwrap_or(""),
                error = %err,
                "request failed with error"
            );
        } else {
            info!(
                event = "request_end",
                request_id = %ctx.request_id,
                method = %method,
                uri = %uri,
                duration_ms = %duration.as_millis(),
                upstream = %ctx.upstream_addr.as_deref().unwrap_or(""),
                "request completed"
            );
        }
    }
}