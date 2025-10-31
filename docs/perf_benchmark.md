# 性能压测方案与验收指标（Pingora 网关）

## 压测目标
- 单机初版：≥ 1K QPS，错误率 ≤ 0.1%
- 调优目标：≥ 10K QPS，P99 ≤ 50ms（参考架构文档）
- 资源约束：CPU/内存/FD 使用率 ≤ 70%

## 压测工具与环境
- 工具：`wrk`、`bombardier`、`k6`
- 环境：本地或容器化；上游服务绑定 `127.0.0.1:8080`（可用简单Echo服务）
- 配置：`TOKIO_WORKER_THREADS = CPU核心数`；KeepAlive 开启

## 启动服务
```bash
# 启动代理层（release）
cargo run --bin proxy --release

# 管理端口（健康与指标）
# health:  http://127.0.0.1:9188/healthz
# metrics: http://127.0.0.1:9188/metrics
```

## 基准场景
### 1) 短连接 vs 长连接
```bash
# 长连接（默认）
wrk -t4 -c100 -d30s --latency http://127.0.0.1:6188/api/test

# 短连接（禁用复用，使用bombardier示例）
bombardier -c 100 -d 30s -l http://127.0.0.1:6188/api/test
```

### 2) HTTP/1.1 vs HTTP/2
```bash
# HTTP/1.1（默认）
bombardier -c 100 -d 30s -l http://127.0.0.1:6188/api/test

# HTTP/2（依客户端支持）
bombardier -c 100 -d 30s -l --http2 http://127.0.0.1:6188/api/test
```

### 3) 热点 vs 均衡路由
```bash
# 单路由热点
wrk -t4 -c200 -d60s --latency http://127.0.0.1:6188/api/hot

# 多路由均衡（k6 示例脚本）
# 参考 docs/tasks.md POC-01/05 场景
```

## 指标采集
- Prometheus 采集：`api_proxy_requests_total`、`api_proxy_upstream_selected_total`、`api_proxy_upstream_errors_total`
- 系统资源：CPU/内存/FD；连接数与端口占用
- 延迟分布：优先观察 P50/P95/P99

## 验收标准
- 吞吐：≥ 1K QPS（初版），≥ 10K QPS（调优）
- 延迟：P99 ≤ 100ms（初版），≤ 50ms（调优）
- 错误率：≤ 0.1%
- 资源：CPU/内存/FD 饱和度 ≤ 70%

## 报告输出
- 测试配置：线程数、连接数、超时、上游数量
- 结果数据：吞吐、延迟分布、错误率、资源使用
- 结论与建议：参数调优建议、瓶颈分析、后续优化项

> 注：与 `docs/mvp_checklist.md`、`docs/tasks.md` 对齐，作为性能交付的基线报告模板。