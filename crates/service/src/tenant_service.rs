use uuid::Uuid;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, Set};
use chrono::Utc;

use models::tenant;
use crate::errors::ServiceError;

/// Create a tenant.
pub async fn create_tenant(db: &DatabaseConnection, name: &str) -> Result<tenant::Model, ServiceError> {
    let created = tenant::create(db, name).await?;
    Ok(created)
}

/// Get tenant by id.
pub async fn get_tenant(db: &DatabaseConnection, id: Uuid) -> Result<Option<tenant::Model>, ServiceError> {
    Ok(tenant::Entity::find_by_id(id).one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?)
}

/// Update tenant name.
pub async fn update_tenant_name(db: &DatabaseConnection, id: Uuid, name: &str) -> Result<tenant::Model, ServiceError> {
    tenant::validate_name(name)?;
    let mut am: tenant::ActiveModel = tenant::Entity::find_by_id(id)
        .one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?
        .ok_or_else(|| ServiceError::not_found("tenant"))?
        .into();
    am.name = Set(name.to_string());
    // no updated_at in schema
    let updated = am.update(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(updated)
}

/// Hard delete tenant.
pub async fn delete_tenant(db: &DatabaseConnection, id: Uuid) -> Result<(), ServiceError> {
    tenant::Entity::delete_by_id(id).exec(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::get_db;

    #[tokio::test]
    async fn tenant_crud_service() -> Result<(), anyhow::Error> {
        if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
        let db = get_db().await?;

        let name = format!("svc_tenant_{}", Uuid::new_v4());
        let t = create_tenant(&db, &name).await?;
        assert_eq!(t.name, name);

        let found = get_tenant(&db, t.id).await?.unwrap();
        assert_eq!(found.id, t.id);

        let updated = update_tenant_name(&db, t.id, "new_name").await?;
        assert_eq!(updated.name, "new_name");

        delete_tenant(&db, t.id).await?;
        let after = get_tenant(&db, t.id).await?;
        assert!(after.is_none());

        Ok(())
    }
}