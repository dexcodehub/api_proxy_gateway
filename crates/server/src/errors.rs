use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;
use tracing::error;

#[derive(Debug)]
pub struct ApiError(pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let msg = self.0;
        let status = StatusCode::BAD_GATEWAY;
        (status, Json(serde_json::json!({"errors": [{"status": status.as_u16(), "title": "Bad Gateway", "detail": msg}]}))).into_response()
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
        (status, Json(serde_json::json!({"errors": [{"status": status.as_u16(), "title": "Startup Error", "detail": msg}]}))).into_response()
    }
}

#[derive(Debug, Serialize)]
pub struct JsonApiErrorBody {
    pub errors: Vec<JsonApiErrorItem>,
}

#[derive(Debug, Serialize)]
pub struct JsonApiErrorItem {
    pub status: u16,
    pub title: String,
    pub detail: Option<String>,
}

/// Standardized JSON:API error type implementing IntoResponse
#[derive(Debug)]
pub struct JsonApiError {
    pub status: StatusCode,
    pub title: String,
    pub detail: Option<String>,
}

impl JsonApiError {
    pub fn new(status: StatusCode, title: impl Into<String>, detail: Option<String>) -> Self {
        Self { status, title: title.into(), detail }
    }
}

impl IntoResponse for JsonApiError {
    fn into_response(self) -> Response {
        let body = JsonApiErrorBody {
            errors: vec![JsonApiErrorItem {
                status: self.status.as_u16(),
                title: self.title,
                detail: self.detail,
            }],
        };
        (self.status, Json(body)).into_response()
    }
}