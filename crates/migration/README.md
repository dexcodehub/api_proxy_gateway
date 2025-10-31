# Running Migrator CLI

- Generate a new migration file
    ```sh
    cargo run -- generate MIGRATION_NAME
    ```
- Apply all pending migrations
    ```sh
    cargo run
    ```
    ```sh
    cargo run -- up
    ```
- Apply first 10 pending migrations
    ```sh
    cargo run -- up -n 10
    ```
- Rollback last applied migrations
    ```sh
    cargo run -- down
    ```
- Rollback last 10 applied migrations
    ```sh
    cargo run -- down -n 10
    ```
- Drop all tables from the database, then reapply all migrations
    ```sh
    cargo run -- fresh
    ```
- Rollback all applied migrations, then reapply all migrations
    ```sh
    cargo run -- refresh
    ```
- Rollback all applied migrations
    ```sh
    cargo run -- reset
    ```
- Check the status of all migrations
    ```sh
    cargo run -- status
    ```

## Notes

- Tables are created per-entity in separate migrations to improve maintainability. The migrator registers them in dependency order, and applies indexes last for better performance.
- For large datasets on Postgres, consider using `CREATE INDEX CONCURRENTLY` (not supported by transactions); SeaORM Migration may not expose this directly. For heavy index builds, schedule during low-traffic windows.
- Logging: the CLI prints progress and errors. Ensure `DATABASE_URL` is set before running. Use environment-specific URLs to avoid accidental migrations in production.
- Compatibility: schemas align with the `models` crate entities (`tenant`, `user`, `api_key`, `upstream`, `rate_limit`, `route`, `request_log`). Run `cargo build -p migration` after changes to validate.
