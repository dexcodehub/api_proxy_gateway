use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::errors::ServiceError;
use crate::storage::json_map_store::JsonMapStore;
use crate::admin::api_mgmt_store::ApiManagementStore;

/// 认证信息定义：目前支持是否需要 API Key，后续可扩展为更多类型
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthInfo {
    pub require_api_key: bool,
}

/// API 记录结构：用于描述被代理/转发的 API
/// - endpoint_url: 例如 `/api/v1/orders`
/// - method: `GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS`
/// - forward_target: 例如 `https://upstream.example.com`
/// - auth: 认证要求，目前仅包含是否需要 API Key
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ApiRecord {
    pub id: Uuid,
    pub endpoint_url: String,
    pub method: String,
    pub forward_target: String,
    pub auth: AuthInfo,
    pub created_at: DateTime<Utc>,
}

/// 创建/更新输入模型：不包含 id/created_at，由服务端生成
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ApiRecordInput {
    pub endpoint_url: String,
    pub method: String,
    pub forward_target: String,
    pub auth: AuthInfo,
}

impl ApiRecordInput {
    /// 统一校验：方法、路径、目标地址
    pub fn validate(&self) -> Result<(), ServiceError> {
        let method_up = self.method.to_ascii_uppercase();
        let valid_methods = [
            "GET","POST","PUT","DELETE","PATCH","HEAD","OPTIONS"
        ];
        if !valid_methods.contains(&method_up.as_str()) {
            return Err(ServiceError::Validation("invalid HTTP method".into()));
        }
        if !(self.endpoint_url.starts_with('/')) {
            return Err(ServiceError::Validation("endpoint_url must start with '/'".into()));
        }
        if !(self.forward_target.starts_with("http://") || self.forward_target.starts_with("https://")) {
            return Err(ServiceError::Validation("forward_target must start with http(s)".into()));
        }
        Ok(())
    }
}

/// 文件存储：以 JSON 文件持久化 API 列表
#[derive(Clone)]
pub struct ApiStore {
    store: Arc<JsonMapStore<Uuid, ApiRecord>>,
}

impl ApiStore {
    /// 初始化存储，若文件不存在则创建空文件
    pub async fn new<P: Into<std::path::PathBuf>>(path: P) -> Result<Arc<Self>, ServiceError> {
        let store = JsonMapStore::<Uuid, ApiRecord>::new(path).await?;
        Ok(Arc::new(Self { store }))
    }

    /// 列出全部 API
    pub async fn list(&self) -> Vec<ApiRecord> {
        self.store
            .list()
            .await
            .into_iter()
            .map(|(_, v)| v)
            .collect()
    }

    /// 根据 id 获取
    pub async fn get(&self, id: Uuid) -> Option<ApiRecord> {
        self.store.get(&id).await
    }

    /// 创建新 API
    pub async fn create(&self, input: ApiRecordInput) -> Result<ApiRecord, ServiceError> {
        input.validate()?;
        let rec = ApiRecord {
            id: Uuid::new_v4(),
            endpoint_url: input.endpoint_url,
            method: input.method.to_ascii_uppercase(),
            forward_target: input.forward_target,
            auth: input.auth,
            created_at: Utc::now(),
        };
        self.store.insert(rec.id, rec.clone()).await?;
        Ok(rec)
    }

    /// 更新指定 API（幂等）
    pub async fn update(&self, id: Uuid, input: ApiRecordInput) -> Result<ApiRecord, ServiceError> {
        input.validate()?;
        let mut updated: Option<ApiRecord> = None;
        self.store
            .update_map(|map| {
                let existed = map.get_mut(&id).ok_or_else(|| ServiceError::not_found("api"))?;
                existed.endpoint_url = input.endpoint_url;
                existed.method = input.method.to_ascii_uppercase();
                existed.forward_target = input.forward_target;
                existed.auth = input.auth;
                updated = Some(existed.clone());
                Ok(())
            })
            .await?;
        Ok(updated.expect("updated set"))
    }

    /// 删除指定 API
    pub async fn delete(&self, id: Uuid) -> Result<bool, ServiceError> {
        self.store.remove(&id).await
    }
}

#[async_trait::async_trait]
impl ApiManagementStore for ApiStore {
    async fn list(&self) -> Vec<ApiRecord> { self.list().await }
    async fn get(&self, id: Uuid) -> Option<ApiRecord> { self.get(id).await }
    async fn create(&self, input: ApiRecordInput) -> Result<ApiRecord, ServiceError> { self.create(input).await }
    async fn update(&self, id: Uuid, input: ApiRecordInput) -> Result<ApiRecord, ServiceError> { self.update(id, input).await }
    async fn delete(&self, id: Uuid) -> Result<bool, ServiceError> { self.delete(id).await }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 使用内存文件路径（测试目录下）
    async fn setup_store() -> Arc<ApiStore> {
        // 使用固定测试文件，避免并发冲突可在 CI 中设置 SKIP 以跳过
        ApiStore::new("data/test_apis.json").await.expect("store init")
    }

    #[tokio::test]
    async fn api_store_crud_and_validation() {
        let store = setup_store().await;
        // create
        let input = ApiRecordInput {
            endpoint_url: "/admin/posts".into(),
            method: "get".into(),
            forward_target: "https://jsonplaceholder.typicode.com".into(),
            auth: AuthInfo { require_api_key: true },
        };
        let created = store.create(input.clone()).await.expect("create ok");
        assert_eq!(created.method, "GET");

        // list
        let list = store.list().await;
        assert!(list.iter().any(|r| r.id == created.id));

        // get
        let found = store.get(created.id).await.expect("found");
        assert_eq!(found.endpoint_url, "/admin/posts");

        // update: 避免移动 `input`，改为克隆字段
        let upd = ApiRecordInput {
            method: "POST".into(),
            endpoint_url: input.endpoint_url.clone(),
            forward_target: input.forward_target.clone(),
            auth: input.auth.clone(),
        };
        let updated = store.update(created.id, upd).await.expect("update ok");
        assert_eq!(updated.method, "POST");

        // delete
        let deleted = store.delete(created.id).await.expect("delete ok");
        assert!(deleted);

        // validation errors：构造变体时不移动 `input`
        let bad = ApiRecordInput {
            endpoint_url: "posts".into(),
            method: input.method.clone(),
            forward_target: input.forward_target.clone(),
            auth: input.auth.clone(),
        };
        assert!(matches!(store.create(bad).await, Err(ServiceError::Validation(_))));
        let bad2 = ApiRecordInput {
            forward_target: "ftp://target".into(),
            endpoint_url: input.endpoint_url.clone(),
            method: input.method.clone(),
            auth: input.auth.clone(),
        };
        assert!(matches!(store.create(bad2).await, Err(ServiceError::Validation(_))));
        let bad3 = ApiRecordInput {
            method: "BAD".into(),
            endpoint_url: input.endpoint_url.clone(),
            forward_target: input.forward_target.clone(),
            auth: input.auth.clone(),
        };
        assert!(matches!(store.create(bad3).await, Err(ServiceError::Validation(_))));
    }
}