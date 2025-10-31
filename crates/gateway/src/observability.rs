use once_cell::sync::Lazy;
use prometheus::{register_histogram, register_int_counter, Encoder, Histogram, IntCounter, TextEncoder};

// Prometheus metrics (default registry)
pub static REQUESTS_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "api_proxy_requests_total",
        "Total requests handled by proxy"
    )
    .expect("register requests_total")
});

pub static UPSTREAM_SELECTED_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "api_proxy_upstream_selected_total",
        "Total upstream selections"
    )
    .expect("register upstream_selected_total")
});

pub static UPSTREAM_ERRORS_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "api_proxy_upstream_errors_total",
        "Total upstream selection errors"
    )
    .expect("register upstream_errors_total")
});

pub static REQUEST_DURATION: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "api_proxy_request_duration_seconds",
        "Request duration in seconds",
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("register request_duration")
});

pub static RATE_LIMITED_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "api_proxy_rate_limited_total",
        "Total requests rejected by rate limiter"
    )
    .expect("register rate_limited_total")
});

pub static CIRCUIT_BREAKER_OPEN_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "api_proxy_circuit_breaker_open_total",
        "Total requests rejected by circuit breaker"
    )
    .expect("register circuit_breaker_open_total")
});

pub static RETRIES_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "api_proxy_retries_total",
        "Total retry attempts"
    )
    .expect("register retries_total")
});

pub fn encode_metrics() -> (axum::http::StatusCode, String) {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("metrics encode error: {e}"),
        );
    }
    (
        axum::http::StatusCode::OK,
        String::from_utf8(buffer).unwrap_or_default(),
    )
}