use uuid::Uuid;
use chrono::Utc;
use sea_orm::{DatabaseConnection, ActiveModelTrait, EntityTrait, Set};
use models::route;
use crate::{errors::ServiceError};
use common::pagination::Pagination;

/// Create a route.
pub async fn create_route(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    method: &str,
    path: &str,
    upstream_id: Uuid,
    timeout_ms: i32,
    retry_max_attempts: i32,
    circuit_breaker_threshold: i32,
    rate_limit_id: Option<Uuid>,
) -> Result<route::Model, ServiceError> {
    // basic validation to strengthen correctness
    let method_up = method.to_ascii_uppercase();
    let valid_methods = ["GET","POST","PUT","DELETE","PATCH","HEAD","OPTIONS"];
    if !valid_methods.contains(&method_up.as_str()) {
        return Err(ServiceError::Validation("invalid HTTP method".into()));
    }
    if !path.starts_with('/') {
        return Err(ServiceError::Validation("route path must start with '/'".into()));
    }
    let am = route::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        method: Set(method_up),
        path: Set(path.to_string()),
        upstream_id: Set(upstream_id),
        timeout_ms: Set(timeout_ms),
        retry_max_attempts: Set(retry_max_attempts),
        circuit_breaker_threshold: Set(circuit_breaker_threshold),
        rate_limit_id: Set(rate_limit_id),
        created_at: Set(Utc::now().into()),
    };
    let model = am.insert(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(model)
}

/// Get route by id.
pub async fn get_route(db: &DatabaseConnection, id: Uuid) -> Result<Option<route::Model>, ServiceError> {
    Ok(route::Entity::find_by_id(id).one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?)
}

/// Update route.
pub async fn update_route(
    db: &DatabaseConnection,
    id: Uuid,
    method: Option<&str>,
    path: Option<&str>,
    timeout_ms: Option<i32>,
    retry_max_attempts: Option<i32>,
    circuit_breaker_threshold: Option<i32>,
    rate_limit_id: Option<Option<Uuid>>,
) -> Result<route::Model, ServiceError> {
    let mut am: route::ActiveModel = route::Entity::find_by_id(id)
        .one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?
        .ok_or_else(|| ServiceError::not_found("route"))?
        .into();
    if let Some(m) = method {
        let m_up = m.to_ascii_uppercase();
        let valid_methods = ["GET","POST","PUT","DELETE","PATCH","HEAD","OPTIONS"];
        if !valid_methods.contains(&m_up.as_str()) { return Err(ServiceError::Validation("invalid HTTP method".into())); }
        am.method = Set(m_up);
    }
    if let Some(p) = path {
        if !p.starts_with('/') { return Err(ServiceError::Validation("route path must start with '/'".into())); }
        am.path = Set(p.to_string());
    }
    if let Some(t) = timeout_ms { am.timeout_ms = Set(t); }
    if let Some(r) = retry_max_attempts { am.retry_max_attempts = Set(r); }
    if let Some(c) = circuit_breaker_threshold { am.circuit_breaker_threshold = Set(c); }
    if let Some(rl) = rate_limit_id { am.rate_limit_id = Set(rl); }
    let updated = am.update(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(updated)
}

/// Delete route.
pub async fn delete_route(db: &DatabaseConnection, id: Uuid) -> Result<(), ServiceError> {
    route::Entity::delete_by_id(id).exec(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(())
}

/// List routes for a tenant with pagination.
pub async fn list_routes_by_tenant_paginated(db: &DatabaseConnection, tenant_id: Uuid, opts: Pagination) -> Result<Vec<route::Model>, ServiceError> {
    use sea_orm::{QueryFilter, ColumnTrait, PaginatorTrait};
    let (page_idx, per_page) = opts.normalize();
    let rows = route::Entity::find()
        .filter(route::Column::TenantId.eq(tenant_id))
        .paginate(db, per_page)
        .fetch_page(page_idx)
        .await
        .map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::{tenant, upstream};
    use crate::test_support::get_db;

    #[tokio::test]
    async fn route_crud_service() -> Result<(), anyhow::Error> {
        if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
        let db = get_db().await?;

        let t = tenant::create(&db, &format!("svc_route_tenant_{}", Uuid::new_v4())).await?;
        let up = upstream::create(&db, &format!("svc_up_{}", Uuid::new_v4()), "https://api.example.com").await?;

        let r = create_route(&db, t.id, "GET", "/svc", up.id, 1000, 2, 5, None).await?;
        let found = get_route(&db, r.id).await?.unwrap();
        assert_eq!(found.path, "/svc");

        let updated = update_route(&db, r.id, Some("POST"), Some("/svc2"), Some(2000), Some(3), Some(10), Some(None)).await?;
        assert_eq!(updated.method, "POST");
        assert_eq!(updated.path, "/svc2");

        // pagination
        let page1 = list_routes_by_tenant_paginated(&db, t.id, Pagination { page: 1, per_page: 10 }).await?;
        assert!(!page1.is_empty());

        delete_route(&db, r.id).await?;
        let after = get_route(&db, r.id).await?;
        assert!(after.is_none());

        upstream::Entity::delete_by_id(up.id).exec(&db).await?;
        tenant::Entity::delete_by_id(t.id).exec(&db).await?;

        Ok(())
    }
}