# API Proxy 开发工具

.PHONY: help dev test build clean docker setup migrate

# 默认目标
help:
	@echo "可用命令:"
	@echo "  setup     - 初始化开发环境"
	@echo "  dev       - 启动开发服务器"
	@echo "  test      - 运行所有测试"
	@echo "  build     - 构建 Release 版本"
	@echo "  clean     - 清理构建缓存"
	@echo "  migrate   - 运行数据库迁移"
	@echo "  docker    - 启动 Docker 服务"
	@echo "  bench     - 运行性能测试"
	@echo "  lint      - 代码检查"

# 初始化开发环境
setup:
	@echo "🚀 初始化开发环境..."
	@if [ ! -f .env ]; then cp .env.example .env; echo "✅ 创建 .env 文件"; fi
	@cargo install cargo-watch sqlx-cli --no-default-features --features postgres
	@echo "✅ 安装开发工具完成"

# 启动 Docker 服务
docker:
	@echo "🐳 启动 Docker 服务..."
	@docker-compose up -d postgres redis
	@echo "✅ Docker 服务启动完成"

# 运行数据库迁移
migrate:
	@echo "📊 运行数据库迁移..."
	@cd migration && cargo run
	@echo "✅ 数据库迁移完成"

# 开发模式
dev: docker migrate
	@echo "🔥 启动开发服务器..."
	@cargo watch -x "run --bin core"

# 代理开发模式
proxy-dev:
	@echo "🛰️ 启动 Pingora 代理 (热重载)..."
	@cargo watch -x "run --bin proxy"

# 运行测试
test:
	@echo "🧪 运行测试..."
	@cargo test --workspace
	@cargo test --workspace --release

# 代码检查
lint:
	@echo "🔍 代码检查..."
	@cargo fmt --check
	@cargo clippy --workspace -- -D warnings
	@cargo audit

# 构建 Release 版本
build:
	@echo "🏗️  构建 Release 版本..."
	@cargo build --release
	@ls -lh target/release/core

# 性能测试
bench:
	@echo "⚡ 运行性能测试..."
	@cargo bench

# 代理基准测试
proxy-bench:
	@echo "⚡ 运行代理性能基准 (wrk)..."
	@if ! command -v wrk >/dev/null 2>&1; then \
		 echo "请先安装 wrk: brew install wrk"; \
		 exit 1; \
	fi
	@wrk -t4 -c100 -d20s --latency http://127.0.0.1:6188/health || true

# 清理
clean:
	@echo "🧹 清理构建缓存..."
	@cargo clean
	@docker-compose down -v

# 快速验证
check: lint test
	@echo "✅ 代码检查和测试通过"

# 本地压测
stress-test:
	@echo "💪 启动压力测试..."
	@if ! command -v wrk >/dev/null 2>&1; then \
		echo "请先安装 wrk: brew install wrk"; \
		exit 1; \
	fi
	@wrk -t4 -c100 -d30s --latency http://localhost:8080/health

# 监控服务状态
status:
	@echo "📊 服务状态检查..."
	@curl -s http://localhost:8080/health | jq .
	@curl -s http://localhost:9090/metrics | grep -E "^api_proxy_" | head -10

# 查看日志
logs:
	@echo "📋 查看服务日志..."
	@docker-compose logs -f postgres redis

# 重启开发环境
restart: clean docker migrate dev

# 生产构建检查
prod-check: lint test build
	@echo "🚀 生产环境构建检查完成"
	@./target/release/core --version