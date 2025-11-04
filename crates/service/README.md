# Service Crate

This crate hosts reusable service-layer logic shared across binaries:

- `runtime`: environment checks and initialization helpers (`ensure_env`).
- `admin_http`: admin HTTP server with `/healthz` and `/metrics` endpoints.
- Domain services (e.g., `tenant_service`, `route_service`) consumed by gateways/servers.

Design goals:
- Keep binary crates thin by delegating cross-cutting concerns here.
- Avoid circular deps: service depends on `common`, not vice versa.
- Use workspace dependencies for `tokio`, `axum`, `tracing`.

## Storage

To avoid duplicate file-backed persistence code, the crate provides a reusable
`storage::json_map_store::JsonMapStore<K, V>` which persists a small
`HashMap<K, V>` as JSON with simple CRUD helpers (`list`, `get`, `insert`,
`remove`, `contains_value`, `update_map`).

Existing services like `services::admin_kv_store::ApiKeysStore` and
`services::api_management::ApiStore` now reuse this abstraction to keep a
single responsibility per module while reducing boilerplate.

## Services Layout

Services are grouped by technical domain to keep boundaries clear:

- `services/db/`: SeaORM-backed CRUD services
  - `tenant_service`, `user_service`, `upstream_service`, `route_service`,
    `request_log_service`, `ratelimit_service`, `proxy_api_service`
- `services/file/`: File-backed stores
  - `api_management`, `admin_kv_store`

For backward compatibility, the old paths under `service::services::*` are
re-exported, so existing imports remain valid.

Usage:
- Call `service::runtime::ensure_env(&frontend_dir, &data_dir)` during startup.
- Spawn admin server via `service::admin_http::spawn_admin_server(addr, metrics_fn)`.

## API Management (api_management)

File-backed CRUD for forwarding API definitions. Each record includes:
- `endpoint_url`: path like `/api/v1/orders`
- `method`: `GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS`
- `forward_target`: upstream base, e.g. `https://jsonplaceholder.typicode.com`
- `auth`: `{ require_api_key: bool }`

Example:
```rust
use service::api_management::{ApiStore, ApiRecordInput, AuthInfo};

# async fn demo() -> anyhow::Result<()> {
let store = ApiStore::new("data/apis.json").await?;
let input = ApiRecordInput {
    endpoint_url: "/admin/posts".into(),
    method: "GET".into(),
    forward_target: "https://jsonplaceholder.typicode.com".into(),
    auth: AuthInfo { require_api_key: true },
};
let created = store.create(input).await?;
println!("created {} -> {} {}", created.id, created.method, created.endpoint_url);
# Ok(())
# }
```

Validation and errors:
- 400-like `Validation` if method invalid, path not starting with `/`, or target not `http(s)`
- `Db` for file IO errors (persistence)

Server routes:
- `GET /admin/apis` list, `POST /admin/apis` create
- `GET /admin/apis/:id` read, `PUT /admin/apis/:id` update, `DELETE /admin/apis/:id` delete

## Admin API Keys (admin_kv_store)

Simple file-backed key-value store mapping `user -> api_key`, implemented on
top of `JsonMapStore<String, String>`. Provides `list`, `set`, `delete`, and
`contains_value` with atomic persistence.