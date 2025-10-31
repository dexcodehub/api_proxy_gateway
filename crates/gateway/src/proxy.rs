use async_trait::async_trait;
use arc_swap::ArcSwap;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_core::Result;
use pingora_http::RequestHeader;
use pingora_load_balancing::selection::RoundRobin;
use pingora_load_balancing::LoadBalancer;
use pingora_proxy::{ProxyHttp, Session};
use std::sync::Arc;
use tracing::{error, info, warn};

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

#[async_trait]
impl ProxyHttp for LB {
    type CTX = std::time::Instant;

    fn new_ctx(&self) -> Self::CTX {
        REQUESTS_TOTAL.inc();
        std::time::Instant::now()
    }

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool> {
        // Check rate limiting
        if !self.rate_limiter.check_rate_limit().await {
            crate::observability::RATE_LIMITED_TOTAL.inc();
            warn!("Request rejected by rate limiter");
            let _ = session.respond_error(429).await;
            return Ok(true);
        }

        // Check circuit breaker
        if !self.circuit_breaker.can_execute().await {
            CIRCUIT_BREAKER_OPEN_TOTAL.inc();
            warn!("Request rejected by circuit breaker");
            let _ = session.respond_error(503).await;
            return Ok(true);
        }

        Ok(false)
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let select_upstream = || async {
            match self.load_balancer.select(b"", 256) {
                Some(upstream) => {
                    UPSTREAM_SELECTED_TOTAL.inc();
                    info!("upstream peer selected: {upstream:?}");
                    let peer = Box::new(HttpPeer::new(upstream, false, String::new()));
                    Ok(peer)
                }
                None => {
                    UPSTREAM_ERRORS_TOTAL.inc();
                    error!("no upstream available");
                    Err(RetryableError::retryable("no upstream available".to_string()))
                }
            }
        };

        match retry_with_policy(&self.retry_policy, select_upstream).await {
            Ok(peer) => {
                self.circuit_breaker.record_success().await;
                Ok(peer)
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                RETRIES_TOTAL.inc();
                error!("Failed to select upstream after retries: {}", e);
                Err(pingora_core::Error::new_str("upstream selection failed"))
            }
        }
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        let config = self.config.load();
        if let Some(first_upstream) = config.upstreams.first() {
            upstream_request.insert_header("Host", first_upstream).unwrap();
        } else {
            upstream_request.insert_header("Host", "127.0.0.1:8080").unwrap();
        }
        Ok(())
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        _upstream_response: &mut pingora_http::ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let duration = ctx.elapsed();
        REQUEST_DURATION.observe(duration.as_secs_f64());
        Ok(())
    }

    async fn logging(
        &self,
        session: &mut Session,
        _e: Option<&pingora_core::Error>,
        ctx: &mut Self::CTX,
    ) {
        let duration = ctx.elapsed();
        info!(
            "Request completed: {} {} - Duration: {:?}",
            session.req_header().method,
            session.req_header().uri,
            duration
        );
    }
}