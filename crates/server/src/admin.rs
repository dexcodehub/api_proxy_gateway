use std::{collections::HashMap, path::PathBuf, sync::Arc};

use axum::{extract::{Path, State, Request}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock};
use axum::middleware::Next;
use axum::response::Response;

#[derive(Clone)]
pub struct ApiKeysStore {
    inner: Arc<RwLock<HashMap<String, String>>>,
    file_path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiKeyRecord {
    pub user: String,
    pub api_key: String,
}

impl ApiKeysStore {
    pub async fn new<P: Into<PathBuf>>(path: P) -> anyhow::Result<Arc<Self>> {
        let file_path = path.into();
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await.ok();
        }

        let map: HashMap<String, String> = match fs::read(&file_path).await {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => {
                // initialize empty file
                let empty: HashMap<String, String> = HashMap::new();
                let _ = fs::write(&file_path, serde_json::to_vec(&empty)?).await;
                empty
            }
        };

        Ok(Arc::new(Self {
            inner: Arc::new(RwLock::new(map)),
            file_path,
        }))
    }

    async fn save(&self) -> anyhow::Result<()> {
        let map = self.inner.read().await;
        let data = serde_json::to_vec(&*map)?;
        fs::write(&self.file_path, data).await?;
        Ok(())
    }
}

pub async fn list_api_keys(State(store): State<Arc<ApiKeysStore>>) -> Json<Vec<ApiKeyRecord>> {
    let map = store.inner.read().await;
    let items = map
        .iter()
        .map(|(user, key)| ApiKeyRecord {
            user: user.clone(),
            api_key: key.clone(),
        })
        .collect::<Vec<_>>();
    Json(items)
}

pub async fn set_api_key(
    State(store): State<Arc<ApiKeysStore>>,
    Json(payload): Json<ApiKeyRecord>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if payload.user.trim().is_empty() || payload.api_key.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut map = store.inner.write().await;
    map.insert(payload.user, payload.api_key);
    drop(map);
    if let Err(_) = store.save().await {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn delete_api_key(
    State(store): State<Arc<ApiKeysStore>>,
    Path(user): Path<String>,
) -> StatusCode {
    let mut map = store.inner.write().await;
    let existed = map.remove(&user).is_some();
    drop(map);
    if store.save().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if existed { StatusCode::NO_CONTENT } else { StatusCode::NOT_FOUND }
}

/// Middleware: require valid X-API-Key (or query `api_key`) for API routes
pub async fn require_api_key(
    State(store): State<Arc<ApiKeysStore>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let key_from_header = req
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let key = if let Some(k) = key_from_header {
        Some(k)
    } else {
        // fallback to query param
        req.uri()
            .query()
            .and_then(|q| {
                q.split('&').find_map(|pair| {
                    let mut it = pair.splitn(2, '=');
                    match (it.next(), it.next()) {
                        (Some("api_key"), Some(v)) => Some(v.to_string()),
                        _ => None,
                    }
                })
            })
    };

    let key = match key {
        Some(k) if !k.trim().is_empty() => k,
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    let map = store.inner.read().await;
    let ok = map.values().any(|v| v == &key);
    drop(map);

    if !ok {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}
