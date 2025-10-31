# Pingora 网关 MVP 开发与交付计划

## 目标与范围
- 核心目标：交付一个基于 Pingora 的稳定可运行的 HTTP 代理/网关 MVP，具备基础路由、负载均衡、健康检查、结构化日志与指标导出能力。
- 范围限定：仅覆盖代理层与基础观测（Tracing/Prometheus），业务层与数据访问保持最小依赖；部署以单机或小规模集群为假设。

## 核心功能与优先级（P0→P2）
- P0（必须）
  - HTTP/1.1&HTTP/2 请求转发（透明代理）
  - Round Robin 负载均衡 + TCP 健康检查
  - 基础路由（前缀/精确匹配，按需）
  - 结构化日志（tracing）与基础指标（Prometheus `/metrics`）
  - 健康探针 `/healthz` 与优雅关闭
- P1（高优先）
  - 限流/熔断/重试（tower 策略，最小可用）
  - 多租户隔离（标头/路径携带租户，限流维度）
  - 配置热更新（ArcSwap 版本化）
- P2（次优先）
  - 上游调用参数优化（超时、重试回退）
  - 基准压测与参数整定（线程/连接池/keepalive）

## 里程碑与时间节点（建议）
- Week 1：骨架搭建与可运行
  - 完成 Pingora 代理服务、LB 与健康检查接入
  - 管理端口（Axum）提供 `/healthz` 与 `/metrics`
  - 编译与基础集成测试通过
- Week 2：观测与稳定性
  - tracing JSON 日志落地，关键埋点
  - Prometheus 指标覆盖：请求数、错误数、上游选择、延迟直方图（初版）
  - 优雅关闭与基本异常处理
- Week 3：限流/熔断/重试与压测
  - 基础限流/熔断策略（按租户/路由）
  - 压测方案执行与参数整定；输出初版性能报告
- Week 4：文档与验收
  - API/运维/性能三类文档完善
  - MVP 验收清单核对与已知问题列单

## 测试方案
- 单元测试：
  - 负载均衡选择与健康检查配置正确性
  - 路由匹配与请求头处理
- 集成测试：
  - 代理转发端到端（本地上游 127.0.0.1:8080）
  - 管理接口 `/healthz` 与 `/metrics` 响应
- 压测：
  - `wrk`/`bombardier` 单机 1K–10K QPS 场景（读多写少）
  - 观测指标采集与 SLO 对比

## 性能与优化指标
- 吞吐：≥ 1K QPS（单机初版），目标 ≥ 10K QPS（调优后）
- 延迟：P50 ≤ 15ms，P95 ≤ 35ms，P99 ≤ 50ms（参考架构文档）
- 资源：CPU/内存/FD 饱和度 ≤ 70%（稳态）
- 连接：HTTP KeepAlive / 连接复用有效；不发生连接池枯竭

## 观测与日志
- Tracing：JSON 结构化，字段含 `trace_id`、租户、路由、状态码、错误码、延迟
- Metrics（Prometheus）：
  - `api_proxy_requests_total`
  - `api_proxy_upstream_selected_total`
  - `api_proxy_upstream_errors_total`
  - `api_proxy_request_duration_seconds`（直方图，初版按粗粒度）

## 交付物与验收
- 可运行的 MVP 二进制（代理服务）与配置示例
- 完整文档：开发计划、本指南、API/运维、性能测试报告
- 测试结果：单元/集成/压测报告；覆盖率与已知问题清单

## 风险与缓解
- 上游异常与回退：启用超时/重试/熔断策略，优雅降级
- 配置错误：配置校验与默认值；最小可用回退与快速回滚
- 性能不达标：分阶段调优（线程/连接/缓存/日志级别），针对热点路径专项优化

> 说明：本计划与 `docs/architecture.md`、`docs/tasks.md`、`docs/mvp_checklist.md` 配套，作为实施的执行蓝图与验收参考。