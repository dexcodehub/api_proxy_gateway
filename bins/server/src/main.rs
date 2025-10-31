use std::net::SocketAddr;

use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, delete},
    Json, Router,
};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use std::env;
use tower_http::{cors::CorsLayer, services::{ServeDir, ServeFile}, trace::TraceLayer};
use axum::middleware;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};
use server::admin;
use core::{posts, types::Health};

#[derive(Debug)]
struct ApiError(String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let msg = self.0;
        let status = StatusCode::BAD_GATEWAY;
        (status, Json(serde_json::json!({"error": msg}))).into_response()
    }
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

async fn get_posts() -> Result<Json<serde_json::Value>, ApiError> {
    let json = posts::fetch_posts().await.map_err(|e| ApiError(e.to_string()))?;
    Ok(Json(json))
}

async fn get_post(Path(id): Path<u32>) -> Result<Json<serde_json::Value>, ApiError> {
    let json = posts::fetch_post(id).await.map_err(|e| ApiError(e.to_string()))?;
    Ok(Json(json))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    // Logging
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();

    let cors = CorsLayer::very_permissive();

    let static_dir = ServeDir::new("frontend").fallback(ServeFile::new("frontend/index.html"));

    // Admin state for API Key management
    let admin_store = admin::ApiKeysStore::new("data/api_keys.json").await?;

    // Public routes (static + health)
    let public = Router::new()
        .nest_service("/", static_dir)
        .route("/health", get(health));

    // Protected API routes
    let api = Router::new()
        .route("/api/posts", get(get_posts))
        .route("/api/posts/:id", get(get_post))
        .route_layer(middleware::from_fn_with_state(
            admin_store.clone(),
            admin::require_api_key,
        ));

    // Admin routes
    let admin_routes = Router::new()
        .route("/admin/api-keys", get(admin::list_api_keys).post(admin::set_api_key))
        .route("/admin/api-keys/:user", delete(admin::delete_api_key));

    // Compose
    let router = public
        .merge(api)
        .merge(admin_routes)
        .with_state(admin_store.clone())
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    // 优先使用 config.toml 的服务绑定配置
    let (host, port) = match configs::load_default() {
        Ok(cfg) => {
            let s = cfg.server;
            (s.host, s.port)
        }
        Err(_) => {
            let host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
            let port = env::var("SERVER_PORT").ok().and_then(|p| p.parse::<u16>().ok()).unwrap_or(8081);
            (host, port)
        }
    };
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    info!(%addr, "starting server crate");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}