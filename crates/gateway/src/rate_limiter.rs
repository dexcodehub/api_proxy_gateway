use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{debug, warn};

#[derive(Debug)]
pub struct TokenBucket {
    capacity: u64,
    tokens: u64,
    refill_rate: u64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(capacity: u64, refill_rate: u64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    pub fn try_acquire(&mut self, tokens: u64) -> bool {
        self.refill();
        
        if self.tokens >= tokens {
            self.tokens -= tokens;
            debug!("Token acquired, remaining: {}", self.tokens);
            true
        } else {
            warn!("Rate limit exceeded, tokens: {}, requested: {}", self.tokens, tokens);
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let tokens_to_add = (elapsed.as_secs_f64() * self.refill_rate as f64) as u64;
        
        if tokens_to_add > 0 {
            self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
            self.last_refill = now;
            debug!("Refilled {} tokens, current: {}", tokens_to_add, self.tokens);
        }
    }
}

#[derive(Clone)]
pub struct RateLimiter {
    bucket: Arc<Mutex<TokenBucket>>,
    enabled: bool,
}

impl RateLimiter {
    pub fn new(requests_per_second: u64, burst_size: u64, enabled: bool) -> Self {
        Self {
            bucket: Arc::new(Mutex::new(TokenBucket::new(burst_size, requests_per_second))),
            enabled,
        }
    }

    pub async fn check_rate_limit(&self) -> bool {
        if !self.enabled {
            return true;
        }

        let mut bucket = self.bucket.lock().await;
        bucket.try_acquire(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_token_bucket_basic() {
        let mut bucket = TokenBucket::new(10, 5);
        
        // Should be able to acquire tokens initially
        assert!(bucket.try_acquire(5));
        assert!(bucket.try_acquire(5));
        
        // Should fail when bucket is empty
        assert!(!bucket.try_acquire(1));
    }

    #[tokio::test]
    async fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(10, 10); // 10 tokens per second
        
        // Drain the bucket
        assert!(bucket.try_acquire(10));
        assert!(!bucket.try_acquire(1));
        
        // Wait for refill
        sleep(Duration::from_millis(1100)).await;
        
        // Should have refilled
        assert!(bucket.try_acquire(10));
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(10, 5, true);
        
        // Should allow initial requests
        assert!(limiter.check_rate_limit().await);
        assert!(limiter.check_rate_limit().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let limiter = RateLimiter::new(1, 1, false);
        
        // Should always allow when disabled
        for _ in 0..100 {
            assert!(limiter.check_rate_limit().await);
        }
    }
}