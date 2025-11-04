use uuid::Uuid;
use sea_orm::{DatabaseConnection, ActiveModelTrait, EntityTrait, Set};
use chrono::Utc;
use models::ratelimit;
use crate::{errors::ServiceError};
use common::pagination::Pagination;

/// Create a rate limit.
pub async fn create_rate_limit(db: &DatabaseConnection, tenant_id: Option<Uuid>, requests_per_minute: i32, burst: i32) -> Result<ratelimit::Model, ServiceError> {
    let am = ratelimit::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        requests_per_minute: Set(requests_per_minute),
        burst: Set(burst),
        created_at: Set(Utc::now().into()),
    };
    Ok(am.insert(db).await.map_err(|e| ServiceError::Db(e.to_string()))?)
}

/// Get rate limit by id.
pub async fn get_rate_limit(db: &DatabaseConnection, id: Uuid) -> Result<Option<ratelimit::Model>, ServiceError> {
    Ok(ratelimit::Entity::find_by_id(id).one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?)
}

/// Update rate limit.
pub async fn update_rate_limit(db: &DatabaseConnection, id: Uuid, requests_per_minute: Option<i32>, burst: Option<i32>, tenant_id: Option<Option<Uuid>>) -> Result<ratelimit::Model, ServiceError> {
    let mut am: ratelimit::ActiveModel = ratelimit::Entity::find_by_id(id)
        .one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?
        .ok_or_else(|| ServiceError::not_found("rate_limit"))?
        .into();
    if let Some(rpm) = requests_per_minute {
        if rpm <= 0 { return Err(ServiceError::Validation("requests_per_minute must be > 0".into())); }
        am.requests_per_minute = Set(rpm);
    }
    if let Some(b) = burst {
        if b < 0 { return Err(ServiceError::Validation("burst must be >= 0".into())); }
        am.burst = Set(b);
    }
    if let Some(t) = tenant_id { am.tenant_id = Set(t); }
    Ok(am.update(db).await.map_err(|e| ServiceError::Db(e.to_string()))?)
}

/// Delete rate limit.
pub async fn delete_rate_limit(db: &DatabaseConnection, id: Uuid) -> Result<(), ServiceError> {
    ratelimit::Entity::delete_by_id(id).exec(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(())
}

/// List rate limits by tenant with pagination.
pub async fn list_rate_limits_by_tenant_paginated(db: &DatabaseConnection, tenant_id: Uuid, opts: Pagination) -> Result<Vec<ratelimit::Model>, ServiceError> {
    use sea_orm::{QueryFilter, ColumnTrait, PaginatorTrait};
    let (page_idx, per_page) = opts.normalize();
    let rows = ratelimit::Entity::find()
        .filter(ratelimit::Column::TenantId.eq(tenant_id))
        .paginate(db, per_page)
        .fetch_page(page_idx)
        .await
        .map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::tenant;
    use crate::test_support::get_db;

    #[tokio::test]
    async fn ratelimit_crud_service() -> Result<(), anyhow::Error> {
        if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
        let db = get_db().await?;

        let t = tenant::create(&db, &format!("svc_rl_tenant_{}", Uuid::new_v4())).await?;
        let rl = create_rate_limit(&db, Some(t.id), 60, 10).await?;
        let found = get_rate_limit(&db, rl.id).await?.unwrap();
        assert_eq!(found.requests_per_minute, 60);

        let updated = update_rate_limit(&db, rl.id, Some(120), Some(20), Some(Some(t.id))).await?;
        assert_eq!(updated.requests_per_minute, 120);
        assert_eq!(updated.burst, 20);

        delete_rate_limit(&db, rl.id).await?;
        let after = get_rate_limit(&db, rl.id).await?;
        assert!(after.is_none());

        tenant::Entity::delete_by_id(t.id).exec(&db).await?;
        Ok(())
    }
}