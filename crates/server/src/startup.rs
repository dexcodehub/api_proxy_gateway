use std::{env, net::SocketAddr, sync::Arc};

use axum::Router;
use common::utils::logging::init_logging_default;
use dotenvy::dotenv;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::routes::{self, auth};
use service::{
    file::{admin_kv_store::ApiKeysStore, api_management::ApiStore},
    runtime,
};

/// Initialize logging via shared common utils
fn init_logging() {
    init_logging_default();
}

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
    let admin_store = ApiKeysStore::new("data/api_keys.json").await?;

    // API 管理存储（文件持久化 data/apis.json）
    let api_store = ApiStore::new("data/apis.json").await?;

    // DB connection
    let db = models::db::connect().await?;

    // JWT secret
    let jwt_secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-me".to_string());
    let state = auth::ServerState {
        db,
        auth: auth::ServerAuthConfig { jwt_secret },
        admin_store: std::sync::Arc::clone(&admin_store),
        api_store: std::sync::Arc::clone(&api_store),
    };

    // Build router
    let cors = build_cors();
    let app: Router = routes::build_router(Arc::clone(&admin_store), cors, state);

    // Bind and serve
    let addr = load_bind_addr()?;
    info!(%addr, "starting server crate");
    println!("starting server crate at {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
