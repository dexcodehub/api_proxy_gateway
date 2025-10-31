# 工程化配置（Cargo 依赖、编译优化、日志/监控、CI/CD）

## Cargo.toml 依赖项建议（Workspace）
> 说明：Pingora目前更常以Git源代码依赖的方式集成；如需仅用Hyper/Axum可暂不引入Pingora。

```toml
[workspace]
members = ["crates/common", "crates/core", "crates/utils", "migration"]
resolver = "2"

[workspace.dependencies]
axum = { version = "0.7", features = ["http2"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal"] }
hyper = { version = "1", features = ["http2", "server", "client"] }
tower = { version = "0.4", features = ["limit", "timeout", "retry", "load-shed"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["serde", "v4"] }
dashmap = "5"
moka = { version = "0.12", features = ["future"] }
arc-swap = "1"

reqwest = { version = "0.12", features = ["json", "gzip", "brotli", "zstd", "http2"] }

sea-orm = { version = "0.12", features = ["sqlx-postgres", "macros", "runtime-tokio-rustls"], default-features = false }
sea-orm-migration = { version = "0.12" }

tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "json"] }
opentelemetry = { version = "0.23", features = ["rt-tokio"] }
opentelemetry-otlp = "0.16"
prometheus = "0.13"

# 如需集成Pingora（Git依赖），可在核心crate内声明：
# pingora = { git = "https://github.com/cloudflare/pingora" }
```

### crates/core/Cargo.toml（示例）
```toml
[package]
name = "core"
version = "0.1.0"
edition = "2021"

[dependencies]
axum.workspace = true
tokio.workspace = true
hyper.workspace = true
tower.workspace = true
serde.workspace = true
serde_json.workspace = true
uuid.workspace = true
dashmap.workspace = true
moka.workspace = true
arc-swap.workspace = true
reqwest.workspace = true
sea-orm.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
opentelemetry.workspace = true
opentelemetry-otlp.workspace = true
prometheus.workspace = true

# 可选（特性）
# pingora = { git = "https://github.com/cloudflare/pingora", optional = true }
[features]
pingora = []
```

## 编译优化参数配置
```toml
[profile.release]
opt-level = 3          # 最大优化
lto = "fat"            # 交叉模块链接时优化
codegen-units = 1      # 提升优化效果（降低并行编译）
panic = "abort"        # 减少二进制体积
strip = true           # 去除符号

[profile.dev]
opt-level = 1
debug = true
incremental = true
```

### 构建环境建议
- RUSTFLAGS：`-C target-cpu=native`（生产镜像如不可用需谨慎评估）
- 构建缓存：启用 `sccache` 加速编译
- 二进制体积：采用 `musl` 目标（如需静态部署），注意TLS/OTel兼容

## 日志与监控方案
- 日志（Tracing）：
  - 格式：JSON结构化；字段包含 `trace_id`、租户、路由、状态码、延迟、错误码
  - 采样：按QPS动态采样（高峰降低非关键日志采样率），错误日志全量
  - 切分与归档：按天/大小轮转；敏感字段脱敏
- 指标（Prometheus）：
  - 核心：`gateway_requests_total`、`gateway_request_duration_seconds`(histogram)、`gateway_errors_total`
  - 资源：`db_pool_usage`、`http_inflight_requests`、`cpu/mem/fd`
  - 租户维度：标签 `tenant_id`、`service_id`，避免过多高基数
- 跟踪（OpenTelemetry）：
  - 采样策略：概率采样 + 错误强制采样；端到端链路追踪（Pingora→Axum→Reqwest→DB）
  - OTLP 输出：导出到 APM/可视化平台（Tempo/Jaeger/Datadog等）

## CI/CD 流水线设计（GitHub Actions 示例）
```yaml
name: ci

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  build_test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Cache cargo
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      - name: Build
        run: cargo build --workspace --verbose
      - name: Test
        run: cargo test --workspace --all-features --verbose
      - name: Clippy
        run: cargo clippy --workspace --all-features -- -D warnings
      - name: Audit (optional)
        run: cargo install cargo-audit && cargo audit || true

  release:
    runs-on: ubuntu-latest
    needs: build_test
    if: github.ref == 'refs/heads/main' && startsWith(github.event.head_commit.message, 'release:')
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build release
        run: cargo build --workspace --release
      - name: Publish artifact
        uses: actions/upload-artifact@v4
        with:
          name: gateway-binaries
          path: target/release/
```

## 运行时配置参数（示例）
- 并发与线程：`TOKIO_WORKER_THREADS`、`RUST_MIN_STACK`
- HTTP 服务：`HTTP_KEEPALIVE`、`READ_TIMEOUT`、`WRITE_TIMEOUT`、`IDLE_TIMEOUT`
- 数据库：`DB_URL`、`DB_POOL_MAX`、`DB_POOL_MIN`、`DB_CONN_TIMEOUT`
- 缓存：`CACHE_TTL_ROUTE`、`CACHE_TTL_AUTHZ`、`CACHE_MAX_ITEMS`
- 上游：`UPSTREAM_TIMEOUT`、`UPSTREAM_POOL_MAX`、`DNS_CACHE_TTL`
- 可观测性：`LOG_LEVEL`、`TRACING_FORMAT=json`、`OTLP_ENDPOINT`、`PROMETHEUS_BIND_ADDR`

## 质量门禁与发布策略
- PR 门禁：编译、测试、Clippy 零警告；安全审计通过
- 发布策略：先 Canary 环境（1–5% 流量），稳定后全量发布
- 回滚：二进制与配置版本化；一键回滚并保留观测数据以复盘