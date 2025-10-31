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