use uuid::Uuid;
use chrono::Utc;
use sea_orm::{DatabaseConnection, ActiveModelTrait, EntityTrait, Set};

use models::user;
use crate::{errors::ServiceError, pagination::Pagination};

/// Create a new user under a tenant.
pub async fn create_user(db: &DatabaseConnection, tenant_id: Uuid, email: &str, name: &str) -> Result<user::Model, ServiceError> {
    let created = user::create(db, tenant_id, email, name).await?;
    Ok(created)
}

/// Get a user by id.
pub async fn get_user(db: &DatabaseConnection, id: Uuid) -> Result<Option<user::Model>, ServiceError> {
    let found = user::Entity::find_by_id(id).one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(found)
}

/// Update a user's name.
pub async fn update_user_name(db: &DatabaseConnection, id: Uuid, name: &str) -> Result<user::Model, ServiceError> {
    user::validate_name(name)?;
    let mut am: user::ActiveModel = user::Entity::find_by_id(id)
        .one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?
        .ok_or_else(|| ServiceError::not_found("user"))?
        .into();
    am.name = Set(name.to_string());
    am.updated_at = Set(Utc::now().into());
    let updated = am.update(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(updated)
}

/// Soft-delete a user (marks deleted_at).
pub async fn soft_delete_user(db: &DatabaseConnection, id: Uuid) -> Result<(), ServiceError> {
    user::soft_delete(db, id).await?;
    Ok(())
}

/// Hard-delete a user (removes record).
pub async fn hard_delete_user(db: &DatabaseConnection, id: Uuid) -> Result<(), ServiceError> {
    user::hard_delete(db, id).await?;
    Ok(())
}

/// List users under a tenant.
pub async fn list_users_by_tenant(db: &DatabaseConnection, tenant_id: Uuid) -> Result<Vec<user::Model>, ServiceError> {
    use sea_orm::{QueryFilter, ColumnTrait};
    let users = user::Entity::find().filter(user::Column::TenantId.eq(tenant_id))
        .all(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(users)
}

/// List users by tenant with pagination.
pub async fn list_users_by_tenant_paginated(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    opts: Pagination,
) -> Result<Vec<user::Model>, ServiceError> {
    use sea_orm::{QueryFilter, ColumnTrait, PaginatorTrait};
    let (page_idx, per_page) = opts.normalize();
    // SeaORM's paginate uses 0-based page index internally via fetch_page
    let users = user::Entity::find()
        .filter(user::Column::TenantId.eq(tenant_id))
        .paginate(db, per_page)
        .fetch_page(page_idx)
        .await
        .map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(users)
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::tenant;
    use crate::test_support::get_db;

    #[tokio::test]
    async fn user_crud_service() -> Result<(), anyhow::Error> {
        if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
        let db = get_db().await?;

        let tname = format!("svc_tenant_{}", Uuid::new_v4());
        let t = tenant::create(&db, &tname).await?;

        let email = format!("svc_{}@example.com", Uuid::new_v4());
        let name = "Svc User";
        let u = create_user(&db, t.id, &email, name).await?;
        assert_eq!(u.email, email);

        let found = get_user(&db, u.id).await?.unwrap();
        assert_eq!(found.id, u.id);

        let updated = update_user_name(&db, u.id, "New Name").await?;
        assert_eq!(updated.name, "New Name");

        soft_delete_user(&db, u.id).await?;
        let after_soft = get_user(&db, u.id).await?.unwrap();
        assert!(after_soft.deleted_at.is_some());

        hard_delete_user(&db, u.id).await?;
        let after_hard = get_user(&db, u.id).await?;
        assert!(after_hard.is_none());

        // pagination test
        let u2 = user::create(&db, t.id, &format!("svc_{}@example.com", Uuid::new_v4()), "User2").await?;
        let u3 = user::create(&db, t.id, &format!("svc_{}@example.com", Uuid::new_v4()), "User3").await?;
        let page = Pagination { page: 1, per_page: 2 };
        let page1 = list_users_by_tenant_paginated(&db, t.id, page).await?;
        assert!(page1.len() <= 2 && page1.len() >= 2);

        user::hard_delete(&db, u2.id).await?;
        user::hard_delete(&db, u3.id).await?;
        tenant::Entity::delete_by_id(t.id).exec(&db).await?;
        Ok(())
    }
}