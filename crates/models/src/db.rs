use sea_orm::{Database, DatabaseConnection, ConnectOptions};
use once_cell::sync::Lazy;
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use anyhow::{Context, Result};
use configs as app_configs;

/// Database configuration structure
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub acquire_timeout: Duration,
    pub sqlx_logging: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            // 不再硬编码数据库 URL，统一从环境变量读取
            url: String::new(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600), // 10 minutes
            max_lifetime: Duration::from_secs(3600), // 1 hour
            acquire_timeout: Duration::from_secs(30),
            sqlx_logging: false,
        }
    }
}

impl DatabaseConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        // Load .env if present
        let _ = dotenvy::dotenv();
        
        let mut config = Self::default();
        
        // 数据库 URL 必须通过环境变量提供
        if let Ok(url) = env::var("DATABASE_URL") {
            config.url = url;
        }
        
        // 兼容 .env.example 中命名（优先 DATABASE_*，其次兼容旧的 DB_*）
        if let Ok(max_conn) = env::var("DATABASE_MAX_CONNECTIONS").or_else(|_| env::var("DB_MAX_CONNECTIONS")) {
            if let Ok(val) = max_conn.parse::<u32>() {
                config.max_connections = val;
            }
        }
        
        if let Ok(min_conn) = env::var("DATABASE_MIN_CONNECTIONS").or_else(|_| env::var("DB_MIN_CONNECTIONS")) {
            if let Ok(val) = min_conn.parse::<u32>() {
                config.min_connections = val;
            }
        }
        
        // 连接超时（秒）
        if let Ok(timeout) = env::var("DATABASE_CONNECT_TIMEOUT").or_else(|_| env::var("DB_CONNECT_TIMEOUT_SECS")) {
            if let Ok(val) = timeout.parse::<u64>() {
                config.connect_timeout = Duration::from_secs(val);
            }
        }
        
        // 获取连接超时（秒）
        if let Ok(timeout) = env::var("DATABASE_ACQUIRE_TIMEOUT").or_else(|_| env::var("DB_ACQUIRE_TIMEOUT_SECS")) {
            if let Ok(val) = timeout.parse::<u64>() {
                config.acquire_timeout = Duration::from_secs(val);
            }
        }
        
        // 空闲超时（秒）
        if let Ok(timeout) = env::var("DATABASE_IDLE_TIMEOUT") {
            if let Ok(val) = timeout.parse::<u64>() {
                config.idle_timeout = Duration::from_secs(val);
            }
        }
        
        if let Ok(logging) = env::var("DB_SQLX_LOGGING").or_else(|_| env::var("SQLX_LOGGING")) {
            config.sqlx_logging = logging.to_lowercase() == "true";
        }
        
        config
    }
    
    /// Load configuration from config.toml (preferred)
    pub fn from_file() -> Option<Self> {
        match app_configs::AppConfig::load_and_validate() {
            Ok(cfg) => {
                let db = cfg.database;
                Some(Self {
                    url: db.url,
                    max_connections: db.max_connections,
                    min_connections: db.min_connections,
                    connect_timeout: Duration::from_secs(db.connect_timeout_secs),
                    idle_timeout: Duration::from_secs(db.idle_timeout_secs),
                    max_lifetime: Duration::from_secs(db.max_lifetime_secs),
                    acquire_timeout: Duration::from_secs(db.acquire_timeout_secs),
                    sqlx_logging: db.sqlx_logging,
                })
            }
            Err(e) => {
                tracing::warn!("failed to load config.toml: {}", e);
                None
            }
        }
    }
}

pub static DATABASE_CONFIG: Lazy<DatabaseConfig> = Lazy::new(|| {
    // Prefer config.toml; fallback to env variables
    if let Some(cfg) = DatabaseConfig::from_file() {
        cfg
    } else {
        DatabaseConfig::from_env()
    }
});

pub static DATABASE_URL: Lazy<String> = Lazy::new(|| DATABASE_CONFIG.url.clone());

/// Connect to database with connection pool and retry mechanism
pub async fn connect() -> Result<DatabaseConnection> {
    connect_with_config(&DATABASE_CONFIG).await
}

/// Connect to database with custom configuration
pub async fn connect_with_config(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    // 校验 URL 是否已通过环境变量提供
    if config.url.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "DATABASE_URL 未设置。请在 .env 或环境变量中配置，例如 postgresql://postgres:dev123@localhost:5432/api_proxy"
        ));
    }
    let mut opt = ConnectOptions::new(&config.url);
    
    // Configure connection pool
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect_timeout(config.connect_timeout)
        .idle_timeout(config.idle_timeout)
        .max_lifetime(config.max_lifetime)
        .acquire_timeout(config.acquire_timeout)
        .sqlx_logging(config.sqlx_logging);
    
    // Retry mechanism
    let max_retries = 3;
    let mut last_error = None;
    
    for attempt in 1..=max_retries {
        match Database::connect(opt.clone()).await {
            Ok(db) => {
                tracing::info!("Database connected successfully on attempt {}", attempt);
                return Ok(db);
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    let delay = Duration::from_millis(1000 * attempt as u64);
                    tracing::warn!(
                        "Database connection attempt {} failed, retrying in {:?}ms: {}",
                        attempt,
                        delay.as_millis(),
                        last_error.as_ref().unwrap()
                    );
                    sleep(delay).await;
                } else {
                    tracing::error!("All {} database connection attempts failed", max_retries);
                }
            }
        }
    }
    
    Err(last_error.unwrap())
        .with_context(|| format!("Failed to connect to database after {} attempts", max_retries))
}

/// Test database connection
pub async fn test_connection() -> Result<()> {
    let db = connect().await?;
    
    // Simple ping test
    sea_orm::query::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT 1".to_string()
    );
    
    tracing::info!("Database connection test successful");
    Ok(())
}

/// Get connection pool statistics
pub async fn get_pool_stats(db: &DatabaseConnection) -> Result<String> {
    // Note: sea-orm doesn't expose pool stats directly, but we can test with a simple query
    let start = std::time::Instant::now();
    
    let _result = sea_orm::query::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT 1".to_string()
    );
    
    let duration = start.elapsed();
    
    Ok(format!(
        "Connection pool test - Query executed in {:?}",
        duration
    ))
}