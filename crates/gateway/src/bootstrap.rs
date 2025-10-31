use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use pingora_core::server::Server;
use pingora_core::services::background::background_service;
use pingora_load_balancing::health_check;
use pingora_load_balancing::selection::RoundRobin;
use pingora_load_balancing::LoadBalancer;
use tracing::{info, warn};
use common::utils::logging::init_logging_default;
use service::admin_http;

use crate::config::ProxyConfig;
use crate::observability;
use crate::proxy::LB;
use crate::rate_limiter::RateLimiter;
use crate::retry::RetryPolicy;
use crate::circuit_breaker::CircuitBreaker;

// admin server spawner moved to service::admin_http

fn init_tracing() { init_logging_default(); }

pub fn run() {
    init_tracing();

    // Load configuration
    let config = ProxyConfig::load_from_file("config.json").unwrap_or_else(|e| {
        warn!("Failed to load config file: {}, using defaults", e);
        ProxyConfig::default()
    });
    info!("Loaded configuration: {:?}", config);

    // Spawn admin server for healthz/metrics
    admin_http::spawn_admin_server("127.0.0.1:9188", observability::encode_metrics);

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