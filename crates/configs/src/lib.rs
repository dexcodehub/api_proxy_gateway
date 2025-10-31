use anyhow::Result;
use serde::Deserialize;
use anyhow::anyhow;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub worker_threads: Option<usize>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { host: "127.0.0.1".into(), port: 8080, worker_threads: Some(4) }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")] 
    pub max_connections: u32,
    #[serde(default = "default_min_connections")] 
    pub min_connections: u32,
    #[serde(default = "default_connect_timeout")] 
    pub connect_timeout_secs: u64,
    #[serde(default = "default_idle_timeout")] 
    pub idle_timeout_secs: u64,
    #[serde(default = "default_max_lifetime")] 
    pub max_lifetime_secs: u64,
    #[serde(default = "default_acquire_timeout")] 
    pub acquire_timeout_secs: u64,
    #[serde(default)]
    pub sqlx_logging: bool,
}

fn default_max_connections() -> u32 { 10 }
fn default_min_connections() -> u32 { 2 }
fn default_connect_timeout() -> u64 { 30 }
fn default_idle_timeout() -> u64 { 600 }
fn default_max_lifetime() -> u64 { 3600 }
fn default_acquire_timeout() -> u64 { 30 }

pub fn load_default() -> Result<AppConfig> {
    let path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());
    load_from_file(&path)
}

pub fn load_from_file(path: &str) -> Result<AppConfig> {
    let content = std::fs::read_to_string(path)?;
    let cfg: AppConfig = toml::from_str(&content)?;
    Ok(cfg)
}

impl AppConfig {
    pub fn load_and_validate() -> Result<Self> {
        let mut cfg = load_default()?;
        cfg.normalize_and_validate()?;
        Ok(cfg)
    }

    pub fn normalize_and_validate(&mut self) -> Result<()> {
        // 归一化 server
        self.server.normalize()?;
        // 归一化 database（支持从环境变量填充 URL）
        self.database.normalize_from_env();
        self.database.validate()?;
        Ok(())
    }
}

impl ServerConfig {
    fn normalize(&mut self) -> Result<()> {
        if self.host.trim().is_empty() {
            self.host = "127.0.0.1".to_string();
        }
        if self.port == 0 || self.port > 65535 {
            return Err(anyhow!("server.port 必须在 1..=65535 范围内"));
        }
        if let Some(w) = self.worker_threads {
            if w == 0 { self.worker_threads = Some(4); }
        } else {
            self.worker_threads = Some(4);
        }
        Ok(())
    }
}

impl DatabaseConfig {
    pub fn normalize_from_env(&mut self) {
        // 若 TOML 中未提供 URL，则尝试从环境变量填充
        if self.url.trim().is_empty() {
            if let Ok(url) = std::env::var("DATABASE_URL") {
                self.url = url;
            }
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.url.trim().is_empty() {
            return Err(anyhow!("database.url 为空；请在 config.toml 或环境变量 DATABASE_URL 中提供"));
        }
        let lower = self.url.to_lowercase();
        if !(lower.starts_with("postgresql://") || lower.starts_with("postgres://")) {
            return Err(anyhow!("database.url 必须以 postgresql:// 或 postgres:// 开头"));
        }
        if self.min_connections == 0 {
            return Err(anyhow!("database.min_connections 必须 >= 1"));
        }
        if self.max_connections < self.min_connections {
            return Err(anyhow!("database.max_connections 必须 >= min_connections"));
        }
        if self.connect_timeout_secs == 0 || self.acquire_timeout_secs == 0 {
            return Err(anyhow!("database 超时配置必须为正整数秒"));
        }
        Ok(())
    }
}
