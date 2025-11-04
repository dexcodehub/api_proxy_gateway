use std::sync::Arc;

use axum::{extract::{Path, State, Request}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use axum::middleware::Next;
use axum::response::Response;
// use proper attribute form: #[utoipa::path] on handlers
use crate::openapi::ApiKeyRecordDoc;
pub use service::admin_kv_store::ApiKeysStore;

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiKeyRecord {
    pub user: String,
    pub api_key: String,
}

#[utoipa::path(get, path = "/admin/api-keys", tag = "admin", responses((status = 200, description = "OK")))]
pub async fn list_api_keys(State(state): State<crate::auth::ServerState>) -> Json<Vec<ApiKeyRecord>> {
    let store = state.admin_store.clone();
    let items = store
        .list()
        .await
        .into_iter()
        .map(|(user, key)| ApiKeyRecord { user, api_key: key })
        .collect::<Vec<_>>();
    Json(items)
}

#[utoipa::path(post, path = "/admin/api-keys", tag = "admin", request_body = ApiKeyRecordDoc, responses((status = 200, description = "OK"), (status = 400, description = "Bad Request")))]
pub async fn set_api_key(
    State(state): State<crate::auth::ServerState>,
    Json(payload): Json<ApiKeyRecord>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let store = state.admin_store.clone();
    if payload.user.trim().is_empty() || payload.api_key.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    if let Err(_) = store.set(payload.user, payload.api_key).await {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn delete_api_key(
    State(state): State<crate::auth::ServerState>,
    Path(user): Path<String>,
) -> StatusCode {
    let store = state.admin_store.clone();
    match store.delete(&user).await {
        Ok(true) => StatusCode::NO_CONTENT,
        Ok(false) => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Middleware: require valid X-API-Key (or query `api_key`) for API routes
pub async fn require_api_key_state(
    State(state): State<crate::auth::ServerState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let store = state.admin_store.clone();
    let key_from_header = req
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let key = if let Some(k) = key_from_header {
        Some(k)
    } else {
        // fallback to query param
        req.uri()
            .query()
            .and_then(|q| {
                q.split('&').find_map(|pair| {
                    let mut it = pair.splitn(2, '=');
                    match (it.next(), it.next()) {
                        (Some("api_key"), Some(v)) => Some(v.to_string()),
                        _ => None,
                    }
                })
            })
    };

    let key = match key {
        Some(k) if !k.trim().is_empty() => k,
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    if !store.contains_value(&key).await {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}
// delete is not documented yet; can be added with #[utoipa::path]
