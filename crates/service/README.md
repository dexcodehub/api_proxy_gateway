# Service Crate

This crate hosts reusable service-layer logic shared across binaries:

- `runtime`: environment checks and initialization helpers (`ensure_env`).
- `admin_http`: admin HTTP server with `/healthz` and `/metrics` endpoints.
- Domain services (e.g., `tenant_service`, `route_service`) consumed by gateways/servers.

Design goals:
- Keep binary crates thin by delegating cross-cutting concerns here.
- Avoid circular deps: service depends on `common`, not vice versa.
- Use workspace dependencies for `tokio`, `axum`, `tracing`.

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