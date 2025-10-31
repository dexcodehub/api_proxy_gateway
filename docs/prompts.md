# Prompts 规范与团队协作指引

## 模块开发提示模板
> 用于与AI/同事协作，高效描述需求与上下文，保证实现一致性与质量。

### 路由模块（Routing）
- 背景：Axum + Pingora，支持多租户与策略匹配（方法/路径/头/查询）
- 目标：在不影响P99延迟的前提下，支持路由动态热更新与缓存命中≥90%
- 输入：路由DSL样例、租户/服务ID、策略（JSON）
- 输出：路由解析器与匹配器实现，带基准测试与指标埋点
- 约束：无阻塞快路径；DashMap/Moka 缓存；Tower 中间件顺序清晰

模板：
```
请基于 Axum + Tower 实现路由解析与匹配：
- 支持 method/path/header/query 多维匹配；
- 路由配置通过 ArcSwap 热更新；
- 本地缓存：DashMap 或 Moka，TTL=60s；
- 提供基准测试（criterion），输出 p50/p95/p99；
- 埋点：tracing span 覆盖匹配耗时与命中结果；
- 返回匹配到的 RouteConfig 与策略集合。
```

### 鉴权模块（AuthN/AuthZ）
模板：
```
实现基于 API Key 的鉴权：
- 输入：请求头 authorization 或 x-api-key；
- 校验：从缓存读取 tenant/scopes，未命中查询数据库；
- 结果：通过返回租户上下文，否则拒绝（401/403）；
- 指标：鉴权命中率、延迟、失败原因分布；
- 安全：密钥只存储哈希，支持轮换与灰度。
```

### 限流/熔断/重试（稳定性）
模板：
```
在 Tower 中实现多租户限流与熔断：
- 限流：令牌桶（租户/路由维度），突发 burst 与 rate_per_sec；
- 熔断：错误比例与并发饱和触发，退避重试≤3次；
- 背压：load_shed/timeout，保护下游；
- 指标：拒绝数、触发次数、恢复时间、尾延迟影响。
```

### 数据访问与缓存
模板：
```
实现 Sea-ORM 仓储与缓存读写：
- 仓储：CRUD + 分页查询，事务封装；
- 缓存：只读快路径，写入通过后台刷新；
- 一致性：版本号/etag 控制，更新时原子替换；
- 指标：池使用率、SQL延迟、缓存命中率。
```

## 代码审查检查清单
- 并发安全：无共享可变数据竞争；使用 Arc/ArcSwap，锁粒度合理
- 性能路径：快路径无阻塞，无不必要分配与拷贝；基准数据达标
- 错误处理：分类清晰（用户错误/系统错误/上游错误），返回码与重试策略合理
- 资源治理：连接池/线程/FD 不发生枯竭；配置参数可调可观测
- 可观测性：日志结构化、span 合理、指标齐全；避免高基数标签
- 安全与隐私：密钥与敏感信息脱敏；最小权限原则；依赖安全审计通过

## 测试用例编写指南
- 单元测试：
  - 路由匹配、鉴权逻辑、限流策略、仓储接口
  - 使用 `#[tokio::test]` 与 `sea-orm` 的 `MockDatabase`（如可用）
- 集成测试：
  - Axum 端到端请求；上游依赖通过 `reqwest::Client` 模拟
  - 数据库使用独立测试库/事务回滚；测试数据构造清晰
- 基准测试：
  - `criterion` 基准：匹配、缓存命中/未命中、序列化/反序列化
- 压测脚本：
  - `wrk`/`bombardier`/`k6` 场景覆盖不同负载模型与响应大小

## 部署配置参数说明
- 服务入口：`BIND_ADDR`、`PORT`、`TLS_CERT_PATH`、`TLS_KEY_PATH`
- 线程与并发：`TOKIO_WORKER_THREADS`、`MAX_INFLIGHT_REQUESTS`
- HTTP：`HTTP_KEEPALIVE`、`READ_TIMEOUT`、`WRITE_TIMEOUT`、`IDLE_TIMEOUT`
- 数据库：`DB_URL`、`DB_POOL_MIN`、`DB_POOL_MAX`、`DB_CONN_TIMEOUT`
- 缓存：`CACHE_TTL_ROUTE`、`CACHE_TTL_AUTHZ`、`CACHE_MAX_ITEMS`
- 上游：`UPSTREAM_TIMEOUT`、`UPSTREAM_POOL_MAX`、`DNS_CACHE_TTL`
- 可观测性：`LOG_LEVEL`、`TRACING_FORMAT`、`OTLP_ENDPOINT`、`PROMETHEUS_BIND_ADDR`
- 可靠性：`RETRY_MAX_ATTEMPTS`、`RETRY_INITIAL_BACKOFF_MS`、`CIRCUIT_BREAKER_THRESHOLD`

## SLO 对齐与发布核对
- QPS/延迟：压测结果与生产指标闭环；预设告警阈值与抖动容忍
- 可用性：故障演练（扩容失败、上游异常、DB不可用）均有可恢复路径
- 配置变更：双人审批、预检查与回滚脚本就绪