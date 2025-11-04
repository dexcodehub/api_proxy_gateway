//! Lightweight admin HTTP server spawner
//!
//! Exposes `/healthz` and `/metrics` endpoints, with metrics provided by caller.

use std::thread;
use axum::{routing::get, Router};
use axum::http::StatusCode;
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use tracing::info;

async fn healthz() -> &'static str { "OK" }

async fn metrics_handler(f: fn() -> (StatusCode, String)) -> (StatusCode, String) {
    f()
}

/// Spawn an admin HTTP server exposing healthz and metrics endpoints.
/// The metrics are provided by the caller via a function.
pub fn spawn_admin_server(addr: &str, metrics_fn: fn() -> (StatusCode, String)) {
    let addr = addr.to_string();
    thread::spawn(move || {
        let rt = Builder::new_multi_thread().enable_all().build().expect("build admin runtime");
        rt.block_on(async move {
            let mf = metrics_fn;
            let router = Router::new()
                .route("/healthz", get(healthz))
                .route("/metrics", get(move || metrics_handler(mf)));
            let listener = TcpListener::bind(&addr).await.expect("bind admin");
            info!(%addr, "admin server listening");
            axum::serve(listener, router).await.expect("serve admin");
        });
    });
}