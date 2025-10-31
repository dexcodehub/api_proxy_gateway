use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub rate_limit: RateLimitConfig,
    pub circuit_breaker: CircuitBreakerConfig,
    pub retry: RetryConfig,
    pub timeout: TimeoutConfig,
    pub upstreams: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_second: u64,
    pub burst_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub enabled: bool,
    pub failure_threshold: u64,
    pub recovery_timeout_secs: u64,
    pub half_open_max_calls: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub enabled: bool,
    pub max_attempts: u32,
    pub backoff_base_ms: u64,
    pub backoff_max_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub connect_timeout_secs: u64,
    pub request_timeout_secs: u64,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            rate_limit: RateLimitConfig {
                enabled: true,
                requests_per_second: 1000,
                burst_size: 100,
            },
            circuit_breaker: CircuitBreakerConfig {
                enabled: true,
                failure_threshold: 5,
                recovery_timeout_secs: 30,
                half_open_max_calls: 3,
            },
            retry: RetryConfig {
                enabled: true,
                max_attempts: 3,
                backoff_base_ms: 100,
                backoff_max_ms: 5000,
            },
            timeout: TimeoutConfig {
                connect_timeout_secs: 5,
                request_timeout_secs: 30,
            },
            upstreams: vec!["127.0.0.1:8080".to_string()],
        }
    }
}

impl ProxyConfig {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: ProxyConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.timeout.connect_timeout_secs)
    }

    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.timeout.request_timeout_secs)
    }

    pub fn recovery_timeout(&self) -> Duration {
        Duration::from_secs(self.circuit_breaker.recovery_timeout_secs)
    }

    pub fn backoff_base(&self) -> Duration {
        Duration::from_millis(self.retry.backoff_base_ms)
    }

    pub fn backoff_max(&self) -> Duration {
        Duration::from_millis(self.retry.backoff_max_ms)
    }
}