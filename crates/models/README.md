# Models

本目录包含与数据库表结构对应的实体与业务方法。以下为新增的被代理 API（Proxy API）模型与表结构说明。

## Proxy API

- 表名：`proxy_api`
- 迁移文件：`crates/migration/src/m20220101_000019_create_proxy_api.rs`

### 字段
- `id` (`uuid`, PK)：主键，手动生成。
- `tenant_id` (`uuid`, FK -> `tenant.id`，`ON DELETE CASCADE`)：所属租户。
- `endpoint_url` (`varchar(256)`，非空)：入口路径，必须以`/`开头，例如`/proxy/posts`。
- `method` (`varchar(16)`，非空)：HTTP 方法，允许值：`GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS`。
- `forward_target` (`varchar(512)`，非空)：目标转发地址，必须以`http://`或`https://`开头。
- `require_api_key` (`bool`，非空)：是否要求 API Key 访问控制。
- `enabled` (`bool`，非空)：是否启用该代理 API。
- `created_at` (`timestamptz`，非空)：创建时间。
- `updated_at` (`timestamptz`，非空)：更新时间。

### 约束与索引
- 复合唯一索引：`(tenant_id, method, endpoint_url)`，防止同一租户重复定义相同入口与方法。
- 外键约束：`tenant_id` 关联 `tenant(id)`，级联更新与删除。

### 模型与验证
文件：`crates/models/src/proxy_api.rs`
- 实体：`proxy_api::Model`（SeaORM `DeriveEntityModel`）。
- 验证方法：
  - `validate_method(m: &str)`：校验并返回大写方法名。
  - `validate_endpoint_url(p: &str)`：要求以`/`开头。
  - `validate_forward_target(u: &str)`：要求以`http(s)://`开头。
- 业务方法：
  - `create(db, tenant_id, endpoint_url, method, forward_target, require_api_key)`：创建记录并默认 `enabled=true`。
  - `set_enabled(db, id, enabled)`：启用/禁用记录，同时更新 `updated_at`。

### 单元测试
文件：`crates/models/src/tests/proxy_api_tests.rs`
- `test_create_and_toggle_proxy_api`：创建并启用切换验证。
- `test_unique_per_tenant`：验证租户内入口与方法复合唯一约束。
- 测试可通过环境变量 `SKIP_DB_TESTS=1` 跳过数据库连接。