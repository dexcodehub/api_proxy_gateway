use uuid::Uuid;
use chrono::Utc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, Set};
use models::upstream;
use crate::{errors::ServiceError, pagination::Pagination};

/// Create an upstream.
pub async fn create_upstream(db: &DatabaseConnection, name: &str, base_url: &str) -> Result<upstream::Model, ServiceError> {
    Ok(upstream::create(db, name, base_url).await?)
}

/// Get upstream by id.
pub async fn get_upstream(db: &DatabaseConnection, id: Uuid) -> Result<Option<upstream::Model>, ServiceError> {
    Ok(upstream::Entity::find_by_id(id).one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?)
}

/// Update upstream info.
pub async fn update_upstream(db: &DatabaseConnection, id: Uuid, name: Option<&str>, base_url: Option<&str>, health_url: Option<&str>, active: Option<bool>) -> Result<upstream::Model, ServiceError> {
    let mut am: upstream::ActiveModel = upstream::Entity::find_by_id(id)
        .one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?
        .ok_or_else(|| ServiceError::not_found("upstream"))?
        .into();
    if let Some(n) = name { am.name = Set(n.to_string()); }
    if let Some(b) = base_url { upstream::validate_base_url(b)?; am.base_url = Set(b.to_string()); }
    if let Some(h) = health_url { am.health_url = Set(Some(h.to_string())); }
    if let Some(a) = active { am.active = Set(a); }
    am.updated_at = Set(Utc::now().into());
    let updated = am.update(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(updated)
}

/// Delete upstream.
pub async fn delete_upstream(db: &DatabaseConnection, id: Uuid) -> Result<(), ServiceError> {
    upstream::Entity::delete_by_id(id).exec(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(())
}

/// List upstreams with optional active filter and pagination.
pub async fn list_upstreams_paginated(db: &DatabaseConnection, active: Option<bool>, opts: Pagination) -> Result<Vec<upstream::Model>, ServiceError> {
    use sea_orm::{QueryFilter, ColumnTrait, PaginatorTrait};
    let (page_idx, per_page) = opts.normalize();
    let mut select = upstream::Entity::find();
    if let Some(a) = active { select = select.filter(upstream::Column::Active.eq(a)); }
    let rows = select
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

    #[tokio::test]
    async fn upstream_crud_service() -> Result<(), anyhow::Error> {
        if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
        let db = get_db().await?;

        let up = create_upstream(&db, &format!("svc_up_{}", Uuid::new_v4()), "https://api.example.com").await?;
        let found = get_upstream(&db, up.id).await?.unwrap();
        assert_eq!(found.id, up.id);

        let updated = update_upstream(&db, up.id, Some("new_name"), Some("https://new.example.com"), Some("https://health.example.com"), Some(false)).await?;
        assert_eq!(updated.name, "new_name");
        assert_eq!(updated.base_url, "https://new.example.com");
        assert_eq!(updated.active, false);

        delete_upstream(&db, up.id).await?;
        let after = get_upstream(&db, up.id).await?;
        assert!(after.is_none());
        Ok(())
    }
}