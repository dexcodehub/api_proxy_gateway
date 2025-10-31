use async_trait::async_trait;
use std::{sync::Arc, thread, time::Duration};

use axum::{routing::get, Router};
use once_cell::sync::Lazy;
use pingora_core::server::Server;
use pingora_core::services::background::background_service;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_core::Result;
use pingora_http::RequestHeader;
use pingora_load_balancing::{health_check, selection::RoundRobin, LoadBalancer};
use pingora_proxy::{ProxyHttp, Session};
use prometheus::{register_int_counter, register_histogram, Encoder, IntCounter, Histogram, TextEncoder};
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};
use arc_swap::ArcSwap;

mod config;
mod rate_limiter;
mod circuit_breaker;
mod retry;

use config::ProxyConfig;
use rate_limiter::RateLimiter;
use circuit_breaker::CircuitBreaker;
use retry::{retry_with_policy, RetryPolicy, RetryableError};

// Minimal Pingora HTTP proxy with RoundRobin load balancing and observability.
// Upstreams: local backends (HTTP). Demonstrates HTTP/1.1 & HTTP/2 proxying.

// Prometheus metrics (default registry)
static REQUESTS_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "api_proxy_requests_total",
        "Total requests handled by proxy"
    )
    .expect("register requests_total")
});

static UPSTREAM_SELECTED_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "api_proxy_upstream_selected_total",
        "Total upstream selections"
    )
    .expect("register upstream_selected_total")
});

static UPSTREAM_ERRORS_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "api_proxy_upstream_errors_total",
        "Total upstream selection errors"
    )
    .expect("register upstream_errors_total")
});

static REQUEST_DURATION: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "api_proxy_request_duration_seconds",
        "Request duration in seconds",
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("register request_duration")
});

static RATE_LIMITED_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "api_proxy_rate_limited_total",
        "Total requests rejected by rate limiter"
    )
    .expect("register rate_limited_total")
});

static CIRCUIT_BREAKER_OPEN_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "api_proxy_circuit_breaker_open_total",
        "Total requests rejected by circuit breaker"
    )
    .expect("register circuit_breaker_open_total")
});

static RETRIES_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "api_proxy_retries_total",
        "Total retry attempts"
    )
    .expect("register retries_total")
});
pub struct LB {
    load_balancer: Arc<LoadBalancer<RoundRobin>>,
    rate_limiter: RateLimiter,
    circuit_breaker: CircuitBreaker,
    retry_policy: RetryPolicy,
    config: Arc<ArcSwap<ProxyConfig>>,
}

#[async_trait]
impl ProxyHttp for LB {
    type CTX = std::time::Instant;

    fn new_ctx(&self) -> Self::CTX {
        REQUESTS_TOTAL.inc();
        std::time::Instant::now()
    }

    async fn request_filter(&self, _session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool> {
        // Check rate limiting
        if !self.rate_limiter.check_rate_limit().await {
            RATE_LIMITED_TOTAL.inc();
            warn!("Request rejected by rate limiter");
            // Return 429 Too Many Requests and stop further processing
            let _ = _session.respond_error(429).await;
            return Ok(true); // We already sent a response
        }

        // Check circuit breaker
        if !self.circuit_breaker.can_execute().await {
            CIRCUIT_BREAKER_OPEN_TOTAL.inc();
            warn!("Request rejected by circuit breaker");
            // Return 503 Service Unavailable and stop further processing
            let _ = _session.respond_error(503).await;
            return Ok(true); // We already sent a response
        }

        Ok(false) // Continue to upstream
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
                    // Plain HTTP to backend; no TLS, no SNI required
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

        // Use retry policy for upstream selection
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
        // Get current config
        let config = self.config.load();
        
        // Set Host header for the first upstream (simplified)
        if let Some(first_upstream) = config.upstreams.first() {
            upstream_request
                .insert_header("Host", first_upstream)
                .unwrap();
        } else {
            upstream_request
                .insert_header("Host", "127.0.0.1:8080")
                .unwrap();
        }
        
        Ok(())
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        _upstream_response: &mut pingora_http::ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        // Record request duration
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

async fn healthz() -> &'static str {
    "OK"
}

async fn metrics() -> (axum::http::StatusCode, String) {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        error!("encode metrics error: {e}");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "metrics encode error".to_string(),
        );
    }
    (
        axum::http::StatusCode::OK,
        String::from_utf8(buffer).unwrap_or_default(),
    )
}

fn spawn_admin_server(addr: &str) {
    let addr = addr.to_string();
    thread::spawn(move || {
        let rt = Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("build admin runtime");
        rt.block_on(async move {
            let router = Router::new()
                .route("/healthz", get(healthz))
                .route("/metrics", get(metrics));
            let listener = TcpListener::bind(&addr).await.expect("bind admin");
            info!("admin server listening on {addr}");
            axum::serve(listener, router).await.expect("serve admin");
        });
    });
}

fn main() {
    // Tracing subscriber with env filter
    let _ = fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .try_init();

    // Load configuration
    let config = ProxyConfig::load_from_file("config.json")
        .unwrap_or_else(|e| {
            warn!("Failed to load config file: {}, using defaults", e);
            ProxyConfig::default()
        });
    
    info!("Loaded configuration: {:?}", config);

    // Spawn admin server for healthz/metrics
    spawn_admin_server("127.0.0.1:9188");

    // Create Pingora server process
    let mut server = Server::new(None).expect("init server");
    server.bootstrap();

    // Build upstream list for load balancing from config
    let peers: Vec<std::net::SocketAddr> = config
        .upstreams
        .iter()
        .map(|addr| addr.parse().expect("parse upstream"))
        .collect();

    // Create LoadBalancer with RoundRobin selection and health checks
    let mut load_balancer = LoadBalancer::<RoundRobin>::try_from_iter(peers).expect("create lb");
    let tcp_hc = health_check::TcpHealthCheck::new();
    load_balancer.set_health_check(tcp_hc);
    load_balancer.health_check_frequency = Some(Duration::from_secs(1));

    // Run health check in background and get shared LB handle
    let background = background_service("health check", load_balancer);
    let upstreams = background.task();
    server.add_service(background);

    // Create rate limiter
    let rate_limiter = RateLimiter::new(
        config.rate_limit.requests_per_second,
        config.rate_limit.burst_size,
        config.rate_limit.enabled,
    );

    // Create circuit breaker
    let circuit_breaker = CircuitBreaker::new(
        config.circuit_breaker.failure_threshold,
        config.recovery_timeout(),
        config.circuit_breaker.half_open_max_calls,
        config.circuit_breaker.enabled,
    );

    // Create retry policy
    let retry_policy = RetryPolicy::new(
        config.retry.max_attempts,
        config.backoff_base(),
        config.backoff_max(),
        config.retry.enabled,
    );

    // Create shared config for hot reloading
    let shared_config = Arc::new(ArcSwap::from_pointee(config));

    // Create LB instance with all components
    let lb_service = LB {
        load_balancer: upstreams,
        rate_limiter,
        circuit_breaker,
        retry_policy,
        config: shared_config,
    };

    // Create HTTP proxy service that uses our LB policy
    let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, lb_service);
    proxy_service.add_tcp("0.0.0.0:6188");

    // Host proxy service
    server.add_service(proxy_service);
    server.run_forever();
}