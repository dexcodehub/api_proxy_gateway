use std::sync::Arc;
use crate::errors::ServiceError;
use crate::storage::json_map_store::JsonMapStore;
use crate::admin::kv_store::AdminKvStore;

/// File-backed key-value store for Admin API keys.
/// Keeps a map of `user -> api_key` persisted as JSON.
#[derive(Clone)]
pub struct ApiKeysStore {
    store: Arc<JsonMapStore<String, String>>,
}

impl ApiKeysStore {
    /// Initialize the store from the given file path. Creates the file if missing.
    pub async fn new<P: Into<std::path::PathBuf>>(path: P) -> Result<Arc<Self>, ServiceError> {
        let store = JsonMapStore::<String, String>::new(path).await?;
        Ok(Arc::new(Self { store }))
    }

    /// List all entries as `(user, api_key)` pairs.
    pub async fn list(&self) -> Vec<(String, String)> {
        self.store.list().await
    }

    /// Upsert the API key for a user and persist.
    pub async fn set(&self, user: String, api_key: String) -> Result<(), ServiceError> {
        self.store.insert(user, api_key).await
    }

    /// Delete the API key for a user; returns whether an entry existed.
    pub async fn delete(&self, user: &str) -> Result<bool, ServiceError> {
        self.store.remove(&user.to_string()).await
    }

    /// Check whether any stored API key equals the given value.
    pub async fn contains_value(&self, value: &str) -> bool {
        self.store.contains_value(&value.to_string()).await
    }
}

#[async_trait::async_trait]
impl AdminKvStore for ApiKeysStore {
    async fn list(&self) -> Vec<(String, String)> { self.list().await }
    async fn set(&self, user: String, api_key: String) -> Result<(), ServiceError> { self.set(user, api_key).await }
    async fn delete(&self, user: &str) -> Result<bool, ServiceError> { self.delete(user).await }
    async fn contains_value(&self, value: &str) -> bool { self.contains_value(value).await }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn admin_kv_store_basic_crud() -> Result<(), anyhow::Error> {
        let tmp = std::env::temp_dir().join(format!("svc_admin_keys_{}.json", Uuid::new_v4()));
        let store = ApiKeysStore::new(&tmp).await?;

        // initially empty
        assert_eq!(store.list().await.len(), 0);

        // set and list
        store.set("alice".to_string(), "key1".to_string()).await?;
        store.set("bob".to_string(), "key2".to_string()).await?;
        let list = store.list().await;
        assert_eq!(list.len(), 2);
        assert!(store.contains_value("key1").await);
        assert!(store.contains_value("key2").await);

        // delete
        let existed = store.delete("alice").await?;
        assert!(existed);
        assert!(!store.contains_value("key1").await);

        // reload store from disk to ensure persistence
        let store2 = ApiKeysStore::new(&tmp).await?;
        let list2 = store2.list().await;
        assert_eq!(list2.len(), 1);
        assert!(store2.contains_value("key2").await);

        // cleanup
        let _ = tokio::fs::remove_file(&tmp).await;
        Ok(())
    }
}