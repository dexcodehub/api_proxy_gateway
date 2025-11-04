use std::{collections::HashMap, hash::Hash, path::PathBuf, sync::Arc};
use tokio::{fs, sync::RwLock};

use crate::errors::ServiceError;

/// Generic JSON file-backed key-value map store.
///
/// Persists a `HashMap<K, V>` to a JSON file and provides simple CRUD helpers.
/// Intended for lightweight configuration/state where a database is overkill.
#[derive(Clone)]
pub struct JsonMapStore<K, V> {
    inner: Arc<RwLock<HashMap<K, V>>>,
    file_path: PathBuf,
}

impl<K, V> JsonMapStore<K, V>
where
    K: Eq + Hash + serde::Serialize + serde::de::DeserializeOwned + Clone,
    V: serde::Serialize + serde::de::DeserializeOwned + Clone + PartialEq,
{
    /// Initialize the store from a path. Creates the file with an empty map if missing.
    pub async fn new<P: Into<PathBuf>>(path: P) -> Result<Arc<Self>, ServiceError> {
        let file_path = path.into();
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await.ok();
        }

        let map: HashMap<K, V> = match fs::read(&file_path).await {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => {
                let empty: HashMap<K, V> = HashMap::new();
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

    /// List all entries as `(key, value)` pairs.
    pub async fn list(&self) -> Vec<(K, V)> {
        let map = self.inner.read().await;
        map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Get value by key.
    pub async fn get(&self, key: &K) -> Option<V> {
        let map = self.inner.read().await;
        map.get(key).cloned()
    }

    /// Insert or update a value by key and persist.
    pub async fn insert(&self, key: K, value: V) -> Result<(), ServiceError> {
        let mut map = self.inner.write().await;
        map.insert(key, value);
        drop(map);
        self.save().await
    }

    /// Remove a key and persist; returns whether it existed.
    pub async fn remove(&self, key: &K) -> Result<bool, ServiceError> {
        let mut map = self.inner.write().await;
        let existed = map.remove(key).is_some();
        drop(map);
        self.save().await?;
        Ok(existed)
    }

    /// Check if any value equals the given value.
    pub async fn contains_value(&self, value: &V) -> bool {
        let map = self.inner.read().await;
        map.values().any(|v| v == value)
    }

    /// Apply a mutation to the underlying map and persist atomically.
    pub async fn update_map<F>(&self, f: F) -> Result<(), ServiceError>
    where
        F: FnOnce(&mut HashMap<K, V>) -> Result<(), ServiceError>,
    {
        let mut map = self.inner.write().await;
        f(&mut map)?;
        drop(map);
        self.save().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn json_map_store_crud_persists() -> Result<(), anyhow::Error> {
        let tmp = std::env::temp_dir().join(format!("json_map_store_{}.json", uuid::Uuid::new_v4()));
        let store = JsonMapStore::<String, String>::new(&tmp).await?;

        // initially empty
        assert_eq!(store.list().await.len(), 0);

        // insert and check
        store.insert("a".into(), "1".into()).await?;
        store.insert("b".into(), "2".into()).await?;
        assert!(store.contains_value(&"1".into()).await);
        assert!(store.get(&"a".into()).await.unwrap() == "1");

        // update_map
        store
            .update_map(|m| {
                if let Some(v) = m.get_mut(&"a".to_string()) { *v = "10".into(); }
                Ok(())
            })
            .await?;
        assert_eq!(store.get(&"a".into()).await.unwrap(), "10");

        // remove and reload persistence
        let existed = store.remove(&"b".into()).await?;
        assert!(existed);
        let reloaded = JsonMapStore::<String, String>::new(&tmp).await?;
        let entries = reloaded.list().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(reloaded.get(&"a".into()).await.unwrap(), "10");

        let _ = tokio::fs::remove_file(&tmp).await;
        Ok(())
    }
}