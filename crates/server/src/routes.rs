use std::sync::Arc;

use axum::{
    extract::Path,
    routing::{delete, get, post},
    Json, Router,
};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::{TraceLayer, DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, DefaultOnFailure},
};
use tracing::Level;
use axum::middleware;

use common::{posts, types::Health};

use crate::admin;
use crate::auth::{self, ServerState};
use crate::errors::ApiError;

pub async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

async fn get_posts() -> Result<Json<serde_json::Value>, ApiError> {
    let json = posts::fetch_posts()
        .await
        .map_err(|e| ApiError(e.to_string()))?;
    Ok(Json(json))
}

async fn get_post(Path(id): Path<u32>) -> Result<Json<serde_json::Value>, ApiError> {
    let json = posts::fetch_post(id)
        .await
        .map_err(|e| ApiError(e.to_string()))?;
    Ok(Json(json))
}

/// Build the full application router, including public, protected, and admin routes
pub fn build_router(_admin_store: Arc<admin::ApiKeysStore>, cors: CorsLayer, state: ServerState) -> Router {
    let static_dir = ServeDir::new("frontend").fallback(ServeFile::new("frontend/index.html"));

    // Public routes (static + health)
    let public = Router::new()
        .nest_service("/", static_dir)
        .route("/health", get(health));

    // Protected API routes (API Key required)
    let api = Router::new()
        .route("/api/posts", get(get_posts))
        .route("/api/posts/:id", get(get_post))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            admin::require_api_key_state,
        ))
        .with_state(state.clone());

    // Auth routes (cookie-based)
    let auth_routes = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout));

    // Admin routes
    let admin_routes = Router::new()
        .route("/admin/api-keys", get(admin::list_api_keys).post(admin::set_api_key))
        .route("/admin/api-keys/:user", delete(admin::delete_api_key))
        .with_state(state.clone());

    // Compose
    public
        .merge(api)
        .merge(auth_routes)
        .merge(admin_routes)
        .with_state(state)
        .layer(cors)
        .layer(
            TraceLayer::new_for_http()
                // 每次请求创建 span，包含方法和路径等，日志级别为 INFO
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(Level::INFO)
                        .include_headers(false),
                )
                // 请求到达时打点
                .on_request(
                    DefaultOnRequest::new()
                        .level(Level::INFO),
                )
                // 响应返回时打点，包含状态码与耗时
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .include_headers(false),
                )
                // 失败（5xx 等）时以 ERROR 记录
                .on_failure(
                    DefaultOnFailure::new()
                        .level(Level::ERROR),
                )
        )
}