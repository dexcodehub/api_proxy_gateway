use std::sync::Arc;

use axum::{
    extract::Path,
    routing::{delete, get},
    Json, Router,
};
use tower_http::{cors::CorsLayer, services::{ServeDir, ServeFile}, trace::TraceLayer};
use axum::middleware;

use common::{posts, types::Health};

use crate::admin;
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
pub fn build_router(admin_store: Arc<admin::ApiKeysStore>, cors: CorsLayer) -> Router {
    let static_dir = ServeDir::new("frontend").fallback(ServeFile::new("frontend/index.html"));

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
    public
        .merge(api)
        .merge(admin_routes)
        .with_state(admin_store.clone())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}