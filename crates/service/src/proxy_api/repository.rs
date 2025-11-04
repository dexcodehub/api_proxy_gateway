use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::ServiceError;

#[async_trait]
pub trait ProxyApiRepository: Send + Sync {
    async fn list(&self, tenant_id: Option<Uuid>) -> Result<Vec<models::proxy_api::Model>, ServiceError>;
    async fn create(&self, tenant_id: Uuid, endpoint_url: &str, method: &str, forward_target: &str, require_api_key: bool) -> Result<models::proxy_api::Model, ServiceError>;
    async fn get(&self, id: Uuid) -> Result<Option<models::proxy_api::Model>, ServiceError>;
    async fn update(&self, id: Uuid, endpoint_url: Option<&str>, method: Option<&str>, forward_target: Option<&str>, require_api_key: Option<bool>, enabled: Option<bool>) -> Result<models::proxy_api::Model, ServiceError>;
    async fn delete(&self, id: Uuid) -> Result<bool, ServiceError>;
}

/// SeaORM-backed repository implementation.
pub struct SeaOrmProxyApiRepository {
    pub db: DatabaseConnection,
}

#[async_trait]
impl ProxyApiRepository for SeaOrmProxyApiRepository {
    async fn list(&self, tenant_id: Option<Uuid>) -> Result<Vec<models::proxy_api::Model>, ServiceError> {
        crate::db::proxy_api_service::list_proxy_apis(&self.db, tenant_id).await
    }

    async fn create(&self, tenant_id: Uuid, endpoint_url: &str, method: &str, forward_target: &str, require_api_key: bool) -> Result<models::proxy_api::Model, ServiceError> {
        crate::db::proxy_api_service::create_proxy_api(&self.db, tenant_id, endpoint_url, method, forward_target, require_api_key).await
    }

    async fn get(&self, id: Uuid) -> Result<Option<models::proxy_api::Model>, ServiceError> {
        crate::db::proxy_api_service::get_proxy_api(&self.db, id).await
    }

    async fn update(&self, id: Uuid, endpoint_url: Option<&str>, method: Option<&str>, forward_target: Option<&str>, require_api_key: Option<bool>, enabled: Option<bool>) -> Result<models::proxy_api::Model, ServiceError> {
        crate::db::proxy_api_service::update_proxy_api(&self.db, id, endpoint_url, method, forward_target, require_api_key, enabled).await
    }

    async fn delete(&self, id: Uuid) -> Result<bool, ServiceError> {
        crate::db::proxy_api_service::delete_proxy_api(&self.db, id).await
    }
}