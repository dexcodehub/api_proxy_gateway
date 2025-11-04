use crate::errors::ServiceError;
use async_trait::async_trait;

/// Trait abstraction for Admin API key storage.
/// Implementations can be file-backed, database-backed, or remote KV.
#[async_trait]
pub trait AdminKvStore: Send + Sync {
    async fn list(&self) -> Vec<(String, String)>;
    async fn set(&self, user: String, api_key: String) -> Result<(), ServiceError>;
    async fn delete(&self, user: &str) -> Result<bool, ServiceError>;
    async fn contains_value(&self, value: &str) -> bool;
}