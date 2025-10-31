# MVP 开发环境搭建指南

## 快速启动

### 1. 环境准备
```bash
# 确保 Rust 版本 >= 1.75
rustc --version

# 安装必要工具
cargo install cargo-watch
cargo install sqlx-cli --no-default-features --features postgres

# 启动 PostgreSQL (Docker)
docker run --name api-proxy-db -e POSTGRES_PASSWORD=dev123 -e POSTGRES_DB=api_proxy -p 5432:5432 -d postgres:15
```

### 2. 项目初始化
```bash
# 克隆并进入项目
cd /Users/rooathuang/Documents/GitHub/api_proxy

# 创建环境配置
cp .env.example .env

# 运行数据库迁移
cd migration
cargo run

# 构建项目
cd ..
cargo build
```

### 3. 开发模式启动
```bash
# 启动核心服务 (热重载)
cargo watch -x "run --bin core"

# 另开终端启动代理层
cargo watch -x "run --bin proxy"
```

## MVP 核心功能范围

### Phase 1: 基础代理 (Week 1-2)
- [x] HTTP 请求转发
- [x] 基础路由匹配
- [x] 简单负载均衡 (Round Robin)
- [x] 健康检查
- [x] 基础日志记录

### Phase 2: 认证与限流 (Week 3)
- [x] API Key 认证
- [x] 基础限流 (Token Bucket)
- [x] 租户隔离
- [x] 请求统计

### Phase 3: 稳定性增强 (Week 4)
- [x] 熔断器
- [x] 超时控制
- [x] 重试机制
- [x] 监控指标

## 开发工作流

### 日常开发
```bash
# 1. 拉取最新代码
git pull origin main

# 2. 运行测试
cargo test

# 3. 代码检查
cargo clippy -- -D warnings
cargo fmt --check

# 4. 启动开发服务
make dev  # 或 cargo watch -x run
```

### 功能开发流程
1. 在 `crates/core/src/` 下创建模块
2. 在 `crates/common/src/` 添加共享类型
3. 更新 `migration/src/` 添加数据库变更
4. 编写单元测试和集成测试
5. 更新 API 文档

### 调试技巧
```bash
# 启用详细日志
RUST_LOG=debug cargo run

# 性能分析
cargo build --release
perf record --call-graph=dwarf ./target/release/core
perf report

# 内存检查
valgrind --tool=memcheck ./target/debug/core
```

## 常见问题

### 数据库连接失败
```bash
# 检查 PostgreSQL 状态
docker ps | grep postgres

# 重置数据库
docker rm -f api-proxy-db
docker run --name api-proxy-db -e POSTGRES_PASSWORD=dev123 -e POSTGRES_DB=api_proxy -p 5432:5432 -d postgres:15
```

### 编译错误
```bash
# 清理缓存
cargo clean

# 更新依赖
cargo update

# 检查 Rust 版本
rustup update stable
```

### 性能问题
```bash
# 检查系统资源
htop
iostat -x 1

# 查看连接数
ss -tuln | grep :8080
```

## 开发规范

### 代码结构
```
crates/
├── common/          # 共享类型和工具
├── core/           # 核心业务逻辑
└── utils/          # 工具函数

每个 crate 内部：
src/
├── lib.rs          # 模块导出
├── config.rs       # 配置定义
├── error.rs        # 错误类型
├── handlers/       # 请求处理
├── services/       # 业务服务
└── models/         # 数据模型
```

### 提交规范
```bash
# 功能开发
git commit -m "feat: 添加 API Key 认证模块"

# 问题修复
git commit -m "fix: 修复连接池泄漏问题"

# 文档更新
git commit -m "docs: 更新 MVP 搭建指南"
```

### 测试策略
- 单元测试：每个模块 >80% 覆盖率
- 集成测试：关键路径端到端验证
- 性能测试：核心接口压测验证
- 手动测试：MVP 功能验收

## 部署检查

### 本地验证
```bash
# 构建 Release 版本
cargo build --release

# 运行基准测试
cargo bench

# 检查二进制大小
ls -lh target/release/core

# 验证配置
./target/release/core --check-config
```

### MVP 验收标准
- [ ] 单机 1K QPS 稳定运行
- [ ] P99 延迟 < 100ms
- [ ] 内存使用 < 100MB
- [ ] 支持 10 个上游服务
- [ ] 基础监控指标正常