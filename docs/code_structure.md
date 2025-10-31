# 核心模块代码结构与接口定义

## 项目结构概览

```
api_proxy/
├── crates/
│   ├── common/              # 共享类型和工具
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── types.rs     # 基础类型定义
│   │   │   ├── error.rs     # 统一错误处理
│   │   │   ├── config.rs    # 配置结构体
│   │   │   └── utils.rs     # 工具函数
│   │   └── Cargo.toml
│   ├── core/                # 核心业务逻辑
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── main.rs      # 服务入口
│   │   │   ├── app.rs       # 应用状态管理
│   │   │   ├── handlers/    # HTTP 处理器
│   │   │   ├── services/    # 业务服务层
│   │   │   ├── middleware/  # 中间件
│   │   │   └── models/      # 数据模型
│   │   └── Cargo.toml
│   └── utils/               # 独立工具库
│       ├── src/
│       │   ├── lib.rs
│       │   ├── cache.rs     # 缓存工具
│       │   ├── metrics.rs   # 指标收集
│       │   └── tracing.rs   # 链路追踪
│       └── Cargo.toml
└── migration/               # 数据库迁移
```

## 核心接口定义

### 1. 共享类型 (common/src/types.rs)

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

/// 服务标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceId(pub String);

/// 租户标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub Uuid);

/// API Key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub key_hash: String,
    pub permissions: Vec<Permission>,
    pub rate_limit: RateLimit,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 权限定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    ReadOnly,
    ReadWrite,
    Admin,
    Custom(String),
}

/// 限流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_second: u32,
    pub burst_size: u32,
    pub window_seconds: u32,
}

/// 上游服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamService {
    pub id: ServiceId,
    pub name: String,
    pub endpoints: Vec<Endpoint>,
    pub load_balancer: LoadBalancerType,
    pub health_check: HealthCheckConfig,
    pub timeout: TimeoutConfig,
}

/// 服务端点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub url: String,
    pub weight: u32,
    pub health_status: HealthStatus,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

/// 负载均衡类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancerType {
    RoundRobin,
    WeightedRoundRobin,
    LeastConnections,
    Random,
}

/// 路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    pub id: Uuid,
    pub path_pattern: String,
    pub method: Option<String>,
    pub headers: HashMap<String, String>,
    pub service_id: ServiceId,
    pub priority: i32,
}
```

### 2. 错误处理 (common/src/error.rs)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiProxyError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Rate limit exceeded for tenant {tenant_id}")]
    RateLimitExceeded { tenant_id: String },
    
    #[error("Service {service_id} not found")]
    ServiceNotFound { service_id: String },
    
    #[error("All endpoints for service {service_id} are unhealthy")]
    NoHealthyEndpoints { service_id: String },
    
    #[error("Upstream request failed: {0}")]
    UpstreamError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("HTTP client error: {0}")]
    HttpClientError(#[from] reqwest::Error),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
}

pub type Result<T> = std::result::Result<T, ApiProxyError>;
```

### 3. 核心服务接口 (core/src/services/)

#### 认证服务 (auth_service.rs)
```rust
use async_trait::async_trait;
use common::{ApiKey, TenantId, Result};

#[async_trait]
pub trait AuthService: Send + Sync {
    /// 验证 API Key
    async fn validate_api_key(&self, key: &str) -> Result<ApiKey>;
    
    /// 检查权限
    async fn check_permission(&self, api_key: &ApiKey, resource: &str, action: &str) -> Result<bool>;
    
    /// 刷新 API Key 缓存
    async fn refresh_cache(&self, tenant_id: &TenantId) -> Result<()>;
}

pub struct AuthServiceImpl {
    db_pool: sqlx::PgPool,
    cache: Arc<DashMap<String, ApiKey>>,
}
```

#### 路由服务 (routing_service.rs)
```rust
use async_trait::async_trait;
use common::{RouteRule, ServiceId, Result};
use hyper::{Method, Uri};

#[async_trait]
pub trait RoutingService: Send + Sync {
    /// 匹配路由规则
    async fn match_route(&self, method: &Method, uri: &Uri, headers: &HeaderMap) -> Result<RouteRule>;
    
    /// 获取服务配置
    async fn get_service(&self, service_id: &ServiceId) -> Result<UpstreamService>;
    
    /// 更新路由配置
    async fn update_routes(&self, routes: Vec<RouteRule>) -> Result<()>;
}
```

#### 负载均衡服务 (load_balancer.rs)
```rust
use async_trait::async_trait;
use common::{UpstreamService, Endpoint, Result};

#[async_trait]
pub trait LoadBalancer: Send + Sync {
    /// 选择健康的端点
    async fn select_endpoint(&self, service: &UpstreamService) -> Result<Endpoint>;
    
    /// 更新端点健康状态
    async fn update_health(&self, service_id: &ServiceId, endpoint_url: &str, is_healthy: bool);
    
    /// 获取服务统计信息
    async fn get_stats(&self, service_id: &ServiceId) -> Result<ServiceStats>;
}

#[derive(Debug)]
pub struct ServiceStats {
    pub total_requests: u64,
    pub healthy_endpoints: usize,
    pub average_response_time: f64,
}
```

#### 限流服务 (rate_limiter.rs)
```rust
use async_trait::async_trait;
use common::{TenantId, RateLimit, Result};

#[async_trait]
pub trait RateLimiter: Send + Sync {
    /// 检查是否允许请求
    async fn check_rate_limit(&self, tenant_id: &TenantId, rate_limit: &RateLimit) -> Result<bool>;
    
    /// 获取剩余配额
    async fn get_remaining_quota(&self, tenant_id: &TenantId) -> Result<u32>;
    
    /// 重置限流计数器
    async fn reset_counter(&self, tenant_id: &TenantId) -> Result<()>;
}
```

### 4. HTTP 处理器 (core/src/handlers/)

#### 代理处理器 (proxy_handler.rs)
```rust
use axum::{
    extract::{Request, State},
    response::Response,
    http::StatusCode,
};
use std::sync::Arc;

pub async fn proxy_handler(
    State(app_state): State<Arc<AppState>>,
    request: Request,
) -> Result<Response, StatusCode> {
    // 1. 提取 API Key
    let api_key = extract_api_key(&request)?;
    
    // 2. 认证和授权
    let auth_info = app_state.auth_service.validate_api_key(&api_key).await?;
    
    // 3. 限流检查
    app_state.rate_limiter.check_rate_limit(&auth_info.tenant_id, &auth_info.rate_limit).await?;
    
    // 4. 路由匹配
    let route = app_state.routing_service.match_route(
        request.method(),
        request.uri(),
        request.headers(),
    ).await?;
    
    // 5. 负载均衡
    let service = app_state.routing_service.get_service(&route.service_id).await?;
    let endpoint = app_state.load_balancer.select_endpoint(&service).await?;
    
    // 6. 转发请求
    let response = forward_request(request, &endpoint).await?;
    
    // 7. 记录日志和指标
    record_request_metrics(&auth_info.tenant_id, &route.service_id, &response);
    
    Ok(response)
}

fn extract_api_key(request: &Request) -> Result<String, StatusCode> {
    request
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .ok_or(StatusCode::UNAUTHORIZED)
}
```

### 5. 应用状态管理 (core/src/app.rs)

```rust
use std::sync::Arc;
use sqlx::PgPool;

pub struct AppState {
    pub db_pool: PgPool,
    pub auth_service: Arc<dyn AuthService>,
    pub routing_service: Arc<dyn RoutingService>,
    pub load_balancer: Arc<dyn LoadBalancer>,
    pub rate_limiter: Arc<dyn RateLimiter>,
    pub http_client: reqwest::Client,
    pub metrics: Arc<PrometheusMetrics>,
}

impl AppState {
    pub async fn new(config: &Config) -> Result<Self> {
        let db_pool = create_db_pool(&config.database_url).await?;
        
        let auth_service = Arc::new(AuthServiceImpl::new(db_pool.clone()));
        let routing_service = Arc::new(RoutingServiceImpl::new(db_pool.clone()));
        let load_balancer = Arc::new(RoundRobinLoadBalancer::new());
        let rate_limiter = Arc::new(TokenBucketRateLimiter::new());
        
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
            
        let metrics = Arc::new(PrometheusMetrics::new());
        
        Ok(Self {
            db_pool,
            auth_service,
            routing_service,
            load_balancer,
            rate_limiter,
            http_client,
            metrics,
        })
    }
}
```

## 开发指导原则

### 1. 异步优先
- 所有 I/O 操作使用 async/await
- 避免阻塞操作，使用 `tokio::spawn` 处理 CPU 密集任务
- 合理使用 `Arc` 和 `Mutex`/`RwLock` 管理共享状态

### 2. 错误处理
- 使用 `thiserror` 定义结构化错误
- 在边界处进行错误转换
- 记录错误日志，包含上下文信息

### 3. 性能考虑
- 使用连接池管理数据库连接
- 实现多级缓存（内存 + Redis）
- 避免不必要的序列化/反序列化

### 4. 可观测性
- 为关键路径添加 tracing span
- 记录业务指标（QPS、延迟、错误率）
- 实现健康检查端点

### 5. 测试策略
- 单元测试：每个服务接口
- 集成测试：端到端流程
- 性能测试：关键路径压测