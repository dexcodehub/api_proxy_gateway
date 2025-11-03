use std::io;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize tracing subscriber with sensible defaults and stdout writer.
/// - Respects `RUST_LOG` if set
/// - Falls back to `info,tower_http=info,axum=info`
/// - Writes to stdout to improve visibility in environments that hide stderr
pub fn init_logging_default() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,axum=info"));
    let _ = fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .with_writer(|| io::stdout())
        .try_init();
}

/// Initialize tracing subscriber with JSON structured output.
/// - Respects `RUST_LOG` if set, defaults to `info`
/// - Emits structured JSON logs for better machine parsing
/// - Writes to stdout for consistent container logging behavior
pub fn init_logging_json() {
    // 默认启用 info，并对 gateway::proxy 下的详细请求处理使用 debug 以便可见
    // 可通过 RUST_LOG 覆盖，例如 RUST_LOG=info,gateway::proxy=trace
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,gateway::proxy=debug"));
    let _ = fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .json()
        .with_writer(|| io::stdout())
        .try_init();
}