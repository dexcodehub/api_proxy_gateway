use std::sync::Arc;
use uuid::Uuid;
use tracing::{info, instrument};

use crate::errors::ServiceError;
use crate::proxy_api::repository::ProxyApiRepository;

/// Application service encapsulating proxy API business rules.
/// Handles validations and tenant existence policy at the service layer.
pub struct ProxyApiService<R: ProxyApiRepository> {
    repo: Arc<R>,
}

impl<R: ProxyApiRepository> ProxyApiService<R> {
    pub fn new(repo: Arc<R>) -> Self { Self { repo } }

    pub async fn list(&self, tenant_id: Option<Uuid>) -> Result<Vec<models::proxy_api::Model>, ServiceError> {
        self.repo.list(tenant_id).await
    }

    /// Create with policy: auto-create tenant if missing.
    #[instrument(skip(self), fields(tenant_id = %tenant_id))]
    pub async fn create(
        &self,
        tenant_id: Uuid,
        endpoint_url: &str,
        method: &str,
        forward_target: &str,
        require_api_key: bool,
        db: &sea_orm::DatabaseConnection,
    ) -> Result<models::proxy_api::Model, ServiceError> {
        // Ensure tenant exists; create if not.
        use sea_orm::{EntityTrait, ActiveModelTrait, Set};
        let maybe = models::tenant::Entity::find_by_id(tenant_id).one(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
        if maybe.is_none() {
            let am = models::tenant::ActiveModel { id: Set(tenant_id), name: Set(format!("auto-tenant-{}", tenant_id)), created_at: Set(chrono::Utc::now().into()) };
            am.insert(db).await.map_err(|e| ServiceError::Db(e.to_string()))?;
            info!(tenant_id = %tenant_id, "auto_created_tenant_for_proxy_api");
        }
        self.repo.create(tenant_id, endpoint_url, method, forward_target, require_api_key).await
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<models::proxy_api::Model>, ServiceError> { self.repo.get(id).await }

    pub async fn update(
        &self,
        id: Uuid,
        endpoint_url: Option<&str>,
        method: Option<&str>,
        forward_target: Option<&str>,
        require_api_key: Option<bool>,
        enabled: Option<bool>,
    ) -> Result<models::proxy_api::Model, ServiceError> {
        self.repo.update(id, endpoint_url, method, forward_target, require_api_key, enabled).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<bool, ServiceError> { self.repo.delete(id).await }
}