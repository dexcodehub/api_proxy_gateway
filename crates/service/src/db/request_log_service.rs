use uuid::Uuid;
use sea_orm::{DatabaseConnection, ActiveModelTrait, EntityTrait, Set};
use chrono::Utc;
use models::request_log;
use crate::{errors::ServiceError};
use common::pagination::Pagination;

/// Create a request log entry.
pub async fn create_request_log(
    db: &DatabaseConnection,
    route_id: Uuid,
    api_key_id: Option<Uuid>,
    status_code: i32,
    latency_ms: i32,
    success: bool,
    error_message: Option<String>,
    client_ip: Option<String>,
) -> Result<request_log::Model, ServiceError> {
    let am = request_log::ActiveModel {
        id: Set(0), // auto-increment by DB
        route_id: Set(route_id),
        api_key_id: Set(api_key_id),
        status_code: Set(status_code),
        latency_ms: Set(latency_ms),
        success: Set(success),
        error_message: Set(error_message),
        client_ip: Set(client_ip),
        timestamp: Set(Utc::now().into()),
    };
    Ok(am.insert(db).await.map_err(|e| ServiceError::Db(e.to_string()))?)
}

/// Get request log by id.
pub async fn get_request_log(db: &DatabaseConnection, id: i64) -> Result<Option<request_log::Model>, ServiceError> {
    Ok(request_log::Entity::find_by_id(id).one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?)
}

/// Delete request log.
pub async fn delete_request_log(db: &DatabaseConnection, id: i64) -> Result<(), ServiceError> {
    request_log::Entity::delete_by_id(id).exec(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(())
}

/// List logs by route with pagination.
pub async fn list_logs_by_route_paginated(db: &DatabaseConnection, route_id: Uuid, opts: Pagination) -> Result<Vec<request_log::Model>, ServiceError> {
    use sea_orm::{QueryFilter, ColumnTrait, PaginatorTrait};
    let (page_idx, per_page) = opts.normalize();
    let rows = request_log::Entity::find()
        .filter(request_log::Column::RouteId.eq(route_id))
        .paginate(db, per_page)
        .fetch_page(page_idx)
        .await
        .map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::get_db;
    use models::{tenant, upstream, route};

    #[tokio::test]
    async fn request_log_crud_service() -> Result<(), anyhow::Error> {
        if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
        let db = get_db().await?;

        let t = tenant::create(&db, &format!("svc_rl_tenant_{}", Uuid::new_v4())).await?;
        let up = upstream::create(&db, &format!("svc_up_{}", Uuid::new_v4()), "https://api.example.com").await?;
        let r = route::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(t.id),
            method: Set("GET".into()),
            path: Set("/svc".into()),
            upstream_id: Set(up.id),
            timeout_ms: Set(1000),
            retry_max_attempts: Set(2),
            circuit_breaker_threshold: Set(5),
            rate_limit_id: Set(None),
            created_at: Set(Utc::now().into()),
        }.insert(&db).await?;

        let log = create_request_log(&db, r.id, None, 200, 123, true, None, Some("127.0.0.1".into())).await?;
        let got = get_request_log(&db, log.id).await?.unwrap();
        assert_eq!(got.status_code, 200);

        // pagination
        let page1 = list_logs_by_route_paginated(&db, r.id, Pagination { page: 1, per_page: 10 }).await?;
        assert!(!page1.is_empty());

        delete_request_log(&db, log.id).await?;
        let after = get_request_log(&db, log.id).await?;
        assert!(after.is_none());

        route::Entity::delete_by_id(r.id).exec(&db).await?;
        upstream::Entity::delete_by_id(up.id).exec(&db).await?;
        tenant::Entity::delete_by_id(t.id).exec(&db).await?;
        Ok(())
    }
}