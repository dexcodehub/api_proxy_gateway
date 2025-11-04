use common::pagination::Pagination;
use uuid::Uuid;
use sea_orm::{DatabaseConnection, EntityTrait};
use models::apikey;
use crate::{errors::ServiceError};

/// Create API key for a user.
pub async fn create_api_key(db: &DatabaseConnection, user_id: Uuid, key_hash: &str) -> Result<apikey::Model, ServiceError> {
    Ok(apikey::create(db, user_id, key_hash).await?)
}

/// Get API key by id.
pub async fn get_api_key(db: &DatabaseConnection, id: Uuid) -> Result<Option<apikey::Model>, ServiceError> {
    Ok(apikey::Entity::find_by_id(id).one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?)
}

/// Delete API key.
pub async fn delete_api_key(db: &DatabaseConnection, id: Uuid) -> Result<(), ServiceError> {
    apikey::Entity::delete_by_id(id).exec(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(())
}

/// List API keys for user.
pub async fn list_api_keys_by_user(db: &DatabaseConnection, user_id: Uuid) -> Result<Vec<apikey::Model>, ServiceError> {
    use sea_orm::{QueryFilter, ColumnTrait};
    let keys = apikey::Entity::find().filter(apikey::Column::UserId.eq(user_id))
        .all(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(keys)
}

/// List API keys by user with pagination.
pub async fn list_api_keys_by_user_paginated(db: &DatabaseConnection, user_id: Uuid, opts: Pagination) -> Result<Vec<apikey::Model>, ServiceError> {
    use sea_orm::{QueryFilter, ColumnTrait, PaginatorTrait};
    let (page_idx, per_page) = opts.normalize();
    let rows = apikey::Entity::find()
        .filter(apikey::Column::UserId.eq(user_id))
        .paginate(db, per_page)
        .fetch_page(page_idx)
        .await
        .map_err(|e| ServiceError::Db(e.to_string()))?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::{tenant, user};
    use crate::test_support::get_db;

    #[tokio::test]
    async fn apikey_crud_service() -> Result<(), anyhow::Error> {
        if std::env::var("SKIP_DB_TESTS").is_ok() { return Ok(()); }
        let db = get_db().await?;

        let t = tenant::create(&db, &format!("svc_apikey_tenant_{}", Uuid::new_v4())).await?;
        let u = user::create(&db, t.id, &format!("svc_{}@example.com", Uuid::new_v4()), "User").await?;

        let key = create_api_key(&db, u.id, "0123456789abcd").await?;
        let got = get_api_key(&db, key.id).await?.unwrap();
        assert_eq!(got.id, key.id);

        let listed = list_api_keys_by_user(&db, u.id).await?;
        assert!(listed.iter().any(|k| k.id == key.id));

        delete_api_key(&db, key.id).await?;
        let after = get_api_key(&db, key.id).await?;
        assert!(after.is_none());

        user::hard_delete(&db, u.id).await?;
        tenant::Entity::delete_by_id(t.id).exec(&db).await?;
        Ok(())
    }
}