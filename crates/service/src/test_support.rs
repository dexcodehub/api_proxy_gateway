#![cfg(test)]
use tokio::sync::OnceCell;
use sea_orm::DatabaseConnection;
use migration::MigratorTrait;
use models::db::{connect_with_config, DatabaseConfig};

// Ensure migrations run only once across the entire test process
static MIGRATED: OnceCell<()> = OnceCell::const_new();

pub async fn get_db() -> Result<DatabaseConnection, anyhow::Error> {
    // Run migrations exactly once, with a throwaway connection
    MIGRATED
        .get_or_init(|| async {
            let mut cfg = DatabaseConfig::from_file().unwrap_or_else(DatabaseConfig::from_env);
            cfg.max_connections = cfg.max_connections.max(10);
            cfg.min_connections = cfg.min_connections.min(1);
            let db = connect_with_config(&cfg).await.expect("connect db for migration");
            migration::Migrator::up(&db, None).await.expect("migrate up");
            drop(db);
        })
        .await;

    // Return a fresh connection for the current test's runtime
    let mut cfg = DatabaseConfig::from_file().unwrap_or_else(DatabaseConfig::from_env);
    cfg.max_connections = cfg.max_connections.max(20);
    cfg.min_connections = cfg.min_connections.min(1);
    cfg.acquire_timeout = std::time::Duration::from_secs(10);
    let db = connect_with_config(&cfg).await?;
    Ok(db)
}