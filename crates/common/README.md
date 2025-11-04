# Common Crate

Shared utilities and types used across the workspace.

- `utils::logging`: centralized logging initialization with sensible defaults.
- `types`: shared DTOs like `Health`, `Post`.
- `crypto`: common cryptographic helpers.
- `pagination`: simple pagination parameters and normalization helpers.
- `env::ensure_env`: runtime environment checks to ensure directories exist.
- `admin_http::spawn_admin_server`: spawns lightweight admin server for health/metrics.

Logging:
- Initialize tracing via `common::utils::logging::init_logging_default("<service>")`.
- Respects `RUST_LOG` or defaults to `info,tower_http=info,axum=info`.
-
Usage examples:
- Pagination
  - `use common::pagination::Pagination;`
  - `let (page_idx, per_page) = Pagination { page: 1, per_page: 20 }.normalize();`
- Env
  - `common::env::ensure_env("frontend", "data").await?;`
- Admin HTTP
  - `common::admin_http::spawn_admin_server("127.0.0.1:9188", metrics_fn);`

Guidelines:
- Keep `common` free of business logic; only pure utilities and types.
- Avoid depending on `service` to prevent cycles.