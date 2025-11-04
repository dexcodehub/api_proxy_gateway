use axum::{extract::{Path, State}, http::StatusCode, Json};
use service::services::api_management::{ApiRecord, ApiRecordInput};
use uuid::Uuid;

use crate::auth::ServerState;
use crate::errors::JsonApiError;

/// 列出所有 API 记录
pub async fn list_apis(State(state): State<ServerState>) -> Json<Vec<ApiRecord>> {
    let store = state.api_store.clone();
    Json(store.list().await)
}

/// 创建 API 记录
pub async fn create_api(
    State(state): State<ServerState>,
    Json(input): Json<ApiRecordInput>,
) -> Result<Json<ApiRecord>, JsonApiError> {
    let store = state.api_store.clone();
    store.create(input).await
        .map(Json)
        .map_err(|e| match e {
            service::errors::ServiceError::Validation(msg) => JsonApiError::new(StatusCode::BAD_REQUEST, "Validation Error", Some(msg)),
            _ => JsonApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error", Some(e.to_string())),
        })
}

/// 获取指定 API 记录
pub async fn get_api(
    State(state): State<ServerState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiRecord>, StatusCode> {
    let store = state.api_store.clone();
    match store.get(id).await {
        Some(rec) => Ok(Json(rec)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// 更新指定 API 记录
pub async fn update_api(
    State(state): State<ServerState>,
    Path(id): Path<Uuid>,
    Json(input): Json<ApiRecordInput>,
) -> Result<Json<ApiRecord>, JsonApiError> {
    let store = state.api_store.clone();
    store.update(id, input).await
        .map(Json)
        .map_err(|e| match e {
            service::errors::ServiceError::Validation(msg) => JsonApiError::new(StatusCode::BAD_REQUEST, "Validation Error", Some(msg)),
            service::errors::ServiceError::NotFound(_) => JsonApiError::new(StatusCode::NOT_FOUND, "Not Found", Some(e.to_string())),
            _ => JsonApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error", Some(e.to_string())),
        })
}

/// 删除指定 API 记录
pub async fn delete_api(
    State(state): State<ServerState>,
    Path(id): Path<Uuid>,
) -> StatusCode {
    let store = state.api_store.clone();
    match store.delete(id).await {
        Ok(true) => StatusCode::NO_CONTENT,
        Ok(false) => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}