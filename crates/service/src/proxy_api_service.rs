use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, Set, QueryFilter, ColumnTrait};
use uuid::Uuid;
use chrono::Utc;
use models::proxy_api::{self, Entity as ProxyApiEntity};
use crate::errors::ServiceError;

/// List proxy APIs, optionally filtered by tenant.
pub async fn list_proxy_apis(db: &DatabaseConnection, tenant_id: Option<Uuid>) -> Result<Vec<proxy_api::Model>, ServiceError> {
    let mut finder = ProxyApiEntity::find();
    if let Some(tid) = tenant_id { finder = finder.filter(proxy_api::Column::TenantId.eq(tid)); }
    let rows = finder.all(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(rows)
}

/// Create a proxy API after validation.
pub async fn create_proxy_api(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    endpoint_url: &str,
    method: &str,
    forward_target: &str,
    require_api_key: bool,
) -> Result<proxy_api::Model, ServiceError> {
    // validations are in models::proxy_api
    let created = proxy_api::create(db, tenant_id, endpoint_url, method, forward_target, require_api_key).await?;
    Ok(created)
}

/// Get a proxy API by id.
pub async fn get_proxy_api(db: &DatabaseConnection, id: Uuid) -> Result<Option<proxy_api::Model>, ServiceError> {
    let found = ProxyApiEntity::find_by_id(id).one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(found)
}

/// Update a proxy API with optional fields and validations.
pub async fn update_proxy_api(
    db: &DatabaseConnection,
    id: Uuid,
    endpoint_url: Option<&str>,
    method: Option<&str>,
    forward_target: Option<&str>,
    require_api_key: Option<bool>,
    enabled: Option<bool>,
) -> Result<proxy_api::Model, ServiceError> {
    let current = ProxyApiEntity::find_by_id(id).one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    let Some(existing) = current else { return Err(ServiceError::not_found("proxy_api")); };
    let mut am: proxy_api::ActiveModel = existing.into();
    if let Some(p) = endpoint_url { proxy_api::validate_endpoint_url(p)?; am.endpoint_url = Set(p.to_string()); }
    if let Some(m) = method { let m2 = proxy_api::validate_method(m)?; am.method = Set(m2); }
    if let Some(u) = forward_target { proxy_api::validate_forward_target(u)?; am.forward_target = Set(u.to_string()); }
    if let Some(b) = require_api_key { am.require_api_key = Set(b); }
    if let Some(b) = enabled { am.enabled = Set(b); }
    am.updated_at = Set(Utc::now().into());
    let updated = am.update(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(updated)
}

/// Delete a proxy API; returns true if deleted.
pub async fn delete_proxy_api(db: &DatabaseConnection, id: Uuid) -> Result<bool, ServiceError> {
    let res = ProxyApiEntity::delete_by_id(id).exec(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(res.rows_affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::get_db;
    use models::tenant;

    #[tokio::test]
    async fn proxy_api_crud_service() -> Result<(), anyhow::Error> {
        if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
        let db = get_db().await?;

        let t = tenant::create(&db, &format!("svc_proxy_tenant_{}", Uuid::new_v4())).await?;

        let a = create_proxy_api(&db, t.id, "/svc/proxy", "GET", "https://api.example.com", false).await?;
        let found = get_proxy_api(&db, a.id).await?.unwrap();
        assert_eq!(found.endpoint_url, "/svc/proxy");
        assert_eq!(found.method, "GET");

        let updated = update_proxy_api(&db, a.id, Some("/svc/proxy2"), Some("POST"), None, Some(true), Some(false)).await?;
        assert_eq!(updated.endpoint_url, "/svc/proxy2");
        assert_eq!(updated.method, "POST");
        assert!(updated.require_api_key);
        assert!(!updated.enabled);

        let list_all = list_proxy_apis(&db, None).await?;
        assert!(!list_all.is_empty());
        let list_tenant = list_proxy_apis(&db, Some(t.id)).await?;
        assert!(list_tenant.iter().any(|x| x.id == a.id));

        let deleted = delete_proxy_api(&db, a.id).await?;
        assert!(deleted);
        let after = get_proxy_api(&db, a.id).await?;
        assert!(after.is_none());

        // cleanup
        tenant::Entity::delete_by_id(t.id).exec(&db).await?;

        Ok(())
    }
}