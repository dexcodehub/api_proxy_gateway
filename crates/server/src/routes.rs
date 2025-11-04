pub mod auth;
pub mod admin;
pub mod apis;
pub mod proxy_apis;

use std::sync::Arc;

use axum::{
    extract::Path,
    routing::{delete, get, post},
    Json, Router,
};
use service::services::admin_kv_store::ApiKeysStore;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::{TraceLayer, DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, DefaultOnFailure},
};
use tracing::Level;
use axum::middleware;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use common::{posts, types::Health};

use self::auth::ServerState;
use crate::errors::ApiError;

#[utoipa::path(get, path = "/health", tag = "health", responses((status = 200, description = "Service OK", body = crate::openapi::HealthResponse)))]
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
pub fn build_router(_admin_store: Arc<ApiKeysStore>, cors: CorsLayer, state: ServerState) -> Router {
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
        // API 管理（CRUD）
        .route("/admin/apis", get(apis::list_apis).post(apis::create_api))
        .route("/admin/apis/:id", get(apis::get_api).put(apis::update_api).delete(apis::delete_api))
        // Proxy API 管理（数据库驱动 CRUD）
        .route("/admin/proxy-apis", get(proxy_apis::list).post(proxy_apis::create))
        .route("/admin/proxy-apis/:id", get(proxy_apis::get).put(proxy_apis::update).delete(proxy_apis::delete))
        .with_state(state.clone());

    // OpenAPI doc
    let openapi = crate::openapi::ApiDoc::openapi();
    let docs = SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi);

    // Compose
    public
        .merge(api)
        .merge(auth_routes)
        .merge(admin_routes)
        .merge(docs)
        .with_state(state.clone())
        // 全局 Bearer Token 校验（白名单在中间件内部）
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_bearer_token_state,
        ))
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