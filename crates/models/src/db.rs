use sea_orm::{Database, DatabaseConnection};
use once_cell::sync::Lazy;
use std::env;

pub static DATABASE_URL: Lazy<String> = Lazy::new(|| {
    // Load .env if present
    let _ = dotenvy::dotenv();
    env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:dev123@localhost:5432/api_proxy".to_string())
});

pub async fn connect() -> anyhow::Result<DatabaseConnection> {
    let db = Database::connect(DATABASE_URL.as_str()).await?;
    Ok(db)
}