use crate::errors::ServiceError;
use async_trait::async_trait;
use uuid::Uuid;

use crate::file::api_management::{ApiRecord, ApiRecordInput};

/// Trait abstraction for API management storage (CRUD of forward/proxy configs).
#[async_trait]
pub trait ApiManagementStore: Send + Sync {
    async fn list(&self) -> Vec<ApiRecord>;
    async fn get(&self, id: Uuid) -> Option<ApiRecord>;
    async fn create(&self, input: ApiRecordInput) -> Result<ApiRecord, ServiceError>;
    async fn update(&self, id: Uuid, input: ApiRecordInput) -> Result<ApiRecord, ServiceError>;
    async fn delete(&self, id: Uuid) -> Result<bool, ServiceError>;
}