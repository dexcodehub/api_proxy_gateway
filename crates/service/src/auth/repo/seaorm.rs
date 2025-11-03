use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter};
use uuid::Uuid;

use crate::auth::domain::{AuthUser, Credentials};
use crate::auth::errors::AuthError;
use crate::auth::repository::AuthRepository;

pub struct SeaOrmAuthRepository {
    pub db: DatabaseConnection,
}

#[async_trait::async_trait]
impl AuthRepository for SeaOrmAuthRepository {
    async fn find_user_by_tenant_email(&self, tenant_id: Uuid, email: &str) -> Result<Option<AuthUser>, AuthError> {
        let res = models::user::Entity::find()
            .filter(models::user::Column::TenantId.eq(tenant_id))
            .filter(models::user::Column::Email.eq(email.to_string()))
            .one(&self.db)
            .await
            .map_err(|e| AuthError::Repository(e.to_string()))?;
        Ok(res.map(|u| AuthUser { id: u.id, tenant_id: u.tenant_id, email: u.email, name: u.name }))
    }

    async fn create_user(&self, tenant_id: Uuid, email: &str, name: &str) -> Result<AuthUser, AuthError> {
        let created = models::user::create(&self.db, tenant_id, email, name)
            .await
            .map_err(|e| AuthError::Validation(e.to_string()))?;
        Ok(AuthUser { id: created.id, tenant_id: created.tenant_id, email: created.email, name: created.name })
    }

    async fn get_credentials(&self, user_id: Uuid) -> Result<Option<Credentials>, AuthError> {
        let res = models::user_credentials::Entity::find()
            .filter(models::user_credentials::Column::UserId.eq(user_id))
            .one(&self.db)
            .await
            .map_err(|e| AuthError::Repository(e.to_string()))?;
        Ok(res.map(|c| Credentials { user_id: c.user_id, password_hash: c.password_hash, password_algorithm: c.password_algorithm }))
    }

    async fn upsert_password(&self, user_id: Uuid, password_hash: String, password_algorithm: String) -> Result<Credentials, AuthError> {
        let c = models::user_credentials::upsert_password(&self.db, user_id, password_hash, &password_algorithm)
            .await
            .map_err(|e| AuthError::Repository(e.to_string()))?;
        Ok(Credentials { user_id: c.user_id, password_hash: c.password_hash, password_algorithm: c.password_algorithm })
    }
}