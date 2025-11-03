use axum::{extract::{Path, Query, State}, http::StatusCode, Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use tracing::{info, error};
use uuid::Uuid;

use crate::auth::ServerState;
use service::proxy_api_service;
use models::tenant;
use sea_orm::{EntityTrait, ActiveModelTrait, Set};
use chrono::Utc;

#[derive(Debug, Deserialize)]
pub struct ListQuery { pub tenant_id: Option<Uuid> }

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProxyApiInput {
    // 前端可留空；后端容错：空或非法时自动生成 UUID
    #[serde(default)]
    pub tenant_id: Option<String>,
    pub endpoint_url: String,
    pub method: String,
    pub forward_target: String,
    #[serde(default)]
    pub require_api_key: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateProxyApiInput {
    pub endpoint_url: Option<String>,
    pub method: Option<String>,
    pub forward_target: Option<String>,
    pub require_api_key: Option<bool>,
    pub enabled: Option<bool>,
}

pub async fn list(State(state): State<ServerState>, Query(q): Query<ListQuery>) -> Result<Json<Vec<models::proxy_api::Model>>, (StatusCode, Json<serde_json::Value>)> {
    match proxy_api_service::list_proxy_apis(&state.db, q.tenant_id).await {
        Ok(list) => { info!(count = list.len(), "list proxy apis"); Ok(Json(list)) }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message": e.to_string()})))),
    }
}

pub async fn create(State(state): State<ServerState>, Json(input): Json<CreateProxyApiInput>) -> Result<Json<models::proxy_api::Model>, (StatusCode, Json<serde_json::Value>)> {
    // 解析/生成租户ID：兼容为空字符串或非法 UUID
    let tid = input
        .tenant_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .unwrap_or_else(uuid::Uuid::new_v4);

    info!(endpoint = %input.endpoint_url, method = %input.method, target = %input.forward_target, require_api_key = %input.require_api_key, tenant_id = %tid, "proxy_api_create_request");

    // Ensure tenant exists; auto-create if missing (align with auth::register behavior)
    let maybe_tenant = tenant::Entity::find_by_id(tid)
        .one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message": e.to_string()}))))?;
    if maybe_tenant.is_none() {
        let am = tenant::ActiveModel {
            id: Set(tid),
            name: Set(format!("auto-tenant-{}", tid)),
            created_at: Set(Utc::now().into()),
        };
        am.insert(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message": e.to_string()}))))?;
        info!(tenant_id = %tid, "auto_created_tenant_for_proxy_api");
    }

    match proxy_api_service::create_proxy_api(&state.db, tid, &input.endpoint_url, &input.method, &input.forward_target, input.require_api_key).await {
        Ok(m) => { info!(id = %m.id, tenant_id = %tid, endpoint = %m.endpoint_url, method = %m.method, "created proxy api"); Ok(Json(m)) },
        Err(e) => {
            match e {
                service::errors::ServiceError::Validation(_) | service::errors::ServiceError::Model(_) => Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"message": e.to_string()})))),
                _ => { error!(err = %e, "create proxy api failed"); Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message": e.to_string()})))) },
            }
        }
    }
}

pub async fn get(State(state): State<ServerState>, Path(id): Path<Uuid>) -> Result<Json<models::proxy_api::Model>, StatusCode> {
    match proxy_api_service::get_proxy_api(&state.db, id).await {
        Ok(Some(m)) => Ok(Json(m)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update(State(state): State<ServerState>, Path(id): Path<Uuid>, Json(input): Json<UpdateProxyApiInput>) -> Result<Json<models::proxy_api::Model>, (StatusCode, Json<serde_json::Value>)> {
    match proxy_api_service::update_proxy_api(
        &state.db,
        id,
        input.endpoint_url.as_deref(),
        input.method.as_deref(),
        input.forward_target.as_deref(),
        input.require_api_key,
        input.enabled,
    ).await {
        Ok(m) => { info!(id = %m.id, "updated proxy api"); Ok(Json(m)) },
        Err(e) => {
            match e {
                service::errors::ServiceError::Validation(_) | service::errors::ServiceError::Model(_) => Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"message": e.to_string()})))),
                service::errors::ServiceError::NotFound(_) => Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"message": e.to_string()})))),
                _ => { error!(err = %e, "update proxy api failed"); Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message": e.to_string()})))) },
            }
        }
    }
}

pub async fn delete(State(state): State<ServerState>, Path(id): Path<Uuid>) -> StatusCode {
    match proxy_api_service::delete_proxy_api(&state.db, id).await {
        Ok(true) => { info!(id = %id, "deleted proxy api"); StatusCode::NO_CONTENT },
        Ok(false) => StatusCode::NOT_FOUND,
        Err(e) => { error!(err = %e, "delete proxy api failed"); StatusCode::INTERNAL_SERVER_ERROR },
    }
}