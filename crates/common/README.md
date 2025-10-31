# Common Crate

Shared utilities and types used across the workspace.

- `utils::logging`: centralized logging initialization with sensible defaults.
- `types`: shared DTOs like `Health`, `Post`.
- `crypto`: common cryptographic helpers.

Logging:
- Initialize tracing via `common::utils::logging::init_logging_default("<service>")`.
- Respects `RUST_LOG` or defaults to `info,tower_http=info,axum=info`.
-
Guidelines:
- Keep `common` free of business logic; only pure utilities and types.
- Avoid depending on `service` to prevent cycles.