use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use thiserror::Error;
use tracing::error;

#[derive(Debug)]
pub struct ApiError(pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let msg = self.0;
        let status = StatusCode::BAD_GATEWAY;
        (status, Json(serde_json::json!({"error": msg}))).into_response()
    }
}

#[derive(Debug, Error)]
pub enum StartupError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("runtime check failed: {0}")]
    Runtime(String),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
}

impl IntoResponse for StartupError {
    fn into_response(self) -> Response {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        let msg = self.to_string();
        error!(error = %msg, "startup error");
        (status, Json(serde_json::json!({"error": msg}))).into_response()
    }
}