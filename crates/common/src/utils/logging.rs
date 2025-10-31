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