use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{fs, sync::RwLock};

use crate::errors::ServiceError;

/// File-backed key-value store for Admin API keys.
/// Keeps a map of `user -> api_key` persisted as JSON.
#[derive(Clone)]
pub struct ApiKeysStore {
    inner: Arc<RwLock<HashMap<String, String>>>,
    file_path: PathBuf,
}

impl ApiKeysStore {
    /// Initialize the store from the given file path. Creates the file if missing.
    pub async fn new<P: Into<PathBuf>>(path: P) -> Result<Arc<Self>, ServiceError> {
        let file_path = path.into();
        if let Some(parent) = file_path.parent() { fs::create_dir_all(parent).await.ok(); }

        let map: HashMap<String, String> = match fs::read(&file_path).await {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => {
                let empty: HashMap<String, String> = HashMap::new();
                fs::write(&file_path, serde_json::to_vec(&empty).map_err(|e| ServiceError::Db(e.to_string()))?)
                    .await
                    .map_err(|e| ServiceError::Db(e.to_string()))?;
                empty
            }
        };

        Ok(Arc::new(Self { inner: Arc::new(RwLock::new(map)), file_path }))
    }

    async fn save(&self) -> Result<(), ServiceError> {
        let map = self.inner.read().await;
        let data = serde_json::to_vec(&*map).map_err(|e| ServiceError::Db(e.to_string()))?;
        fs::write(&self.file_path, data).await.map_err(|e| ServiceError::Db(e.to_string()))?;
        Ok(())
    }

    /// List all entries as `(user, api_key)` pairs.
    pub async fn list(&self) -> Vec<(String, String)> {
        let map = self.inner.read().await;
        map.iter().map(|(u, k)| (u.clone(), k.clone())).collect()
    }

    /// Upsert the API key for a user and persist.
    pub async fn set(&self, user: String, api_key: String) -> Result<(), ServiceError> {
        let mut map = self.inner.write().await;
        map.insert(user, api_key);
        drop(map);
        self.save().await
    }

    /// Delete the API key for a user; returns whether an entry existed.
    pub async fn delete(&self, user: &str) -> Result<bool, ServiceError> {
        let mut map = self.inner.write().await;
        let existed = map.remove(user).is_some();
        drop(map);
        self.save().await?;
        Ok(existed)
    }

    /// Check whether any stored API key equals the given value.
    pub async fn contains_value(&self, value: &str) -> bool {
        let map = self.inner.read().await;
        let ok = map.values().any(|v| v == value);
        ok
    }
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