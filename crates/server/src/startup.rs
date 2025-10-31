use std::{env, net::SocketAddr, sync::Arc};

use axum::Router;
use dotenvy::dotenv;
use tower_http::cors::CorsLayer;
use tracing::info;
use common::utils::logging::init_logging_default;

use crate::{admin, routes};
use service::runtime;

/// Initialize logging via shared common utils
fn init_logging() { init_logging_default(); }

fn build_cors() -> CorsLayer {
    CorsLayer::very_permissive()
}

// runtime checks moved to service::runtime

/// Load host/port from configs or env vars, with sensible fallbacks
fn load_bind_addr() -> anyhow::Result<SocketAddr> {
    let (host, port) = match configs::load_default() {
        Ok(cfg) => {
            let s = cfg.server;
            (s.host, s.port)
        }
        Err(_) => {
            let host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
            let port = env::var("SERVER_PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(8081);
            (host, port)
        }
    };
    Ok(format!("{}:{}", host, port).parse()?)
}

/// Public entry: build the app and run the HTTP server
pub async fn run() -> anyhow::Result<()> {
    dotenv().ok();
    init_logging();

    runtime::ensure_env("frontend", "data").await?;

    // Admin state for API Key management
    let admin_store = admin::ApiKeysStore::new("data/api_keys.json").await?;

    // Build router
    let cors = build_cors();
    let app: Router = routes::build_router(Arc::clone(&admin_store), cors);

    // Bind and serve
    let addr = load_bind_addr()?;
    info!(%addr, "starting server crate");
    println!("starting server crate at {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}