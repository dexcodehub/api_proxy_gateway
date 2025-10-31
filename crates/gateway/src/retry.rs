use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

#[derive(Clone)]
pub struct RetryPolicy {
    max_attempts: u32,
    backoff_base: Duration,
    backoff_max: Duration,
    enabled: bool,
}

impl RetryPolicy {
    pub fn new(
        max_attempts: u32,
        backoff_base: Duration,
        backoff_max: Duration,
        enabled: bool,
    ) -> Self {
        Self {
            max_attempts,
            backoff_base,
            backoff_max,
            enabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn max_attempts(&self) -> u32 {
        if self.enabled {
            self.max_attempts
        } else {
            1
        }
    }

    pub async fn wait_before_retry(&self, attempt: u32) {
        if !self.enabled || attempt == 0 {
            return;
        }

        let backoff_ms = self.backoff_base.as_millis() as u64 * (2_u64.pow(attempt - 1));
        let backoff_duration = Duration::from_millis(backoff_ms.min(self.backoff_max.as_millis() as u64));
        
        debug!("Retrying in {:?} (attempt {})", backoff_duration, attempt);
        sleep(backoff_duration).await;
    }

    pub fn should_retry(&self, attempt: u32, error: &dyn std::error::Error) -> bool {
        if !self.enabled {
            return false;
        }

        if attempt >= self.max_attempts {
            debug!("Max retry attempts ({}) reached", self.max_attempts);
            return false;
        }

        // Check if error is retryable
        let error_str = error.to_string().to_lowercase();
        let is_retryable = error_str.contains("timeout") 
            || error_str.contains("connection") 
            || error_str.contains("network")
            || error_str.contains("temporary")
            || error_str.contains("503")
            || error_str.contains("502")
            || error_str.contains("504");

        if is_retryable {
            debug!("Error is retryable: {}", error);
            true
        } else {
            warn!("Error is not retryable: {}", error);
            false
        }
    }
}

pub struct RetryableError {
    pub message: String,
    pub is_retryable: bool,
}

impl std::fmt::Display for RetryableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::fmt::Debug for RetryableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RetryableError {{ message: {}, is_retryable: {} }}", self.message, self.is_retryable)
    }
}

impl std::error::Error for RetryableError {}

impl RetryableError {
    pub fn new(message: String, is_retryable: bool) -> Self {
        Self { message, is_retryable }
    }

    pub fn retryable(message: String) -> Self {
        Self::new(message, true)
    }

    pub fn non_retryable(message: String) -> Self {
        Self::new(message, false)
    }
}

pub async fn retry_with_policy<F, Fut, T, E>(
    policy: &RetryPolicy,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::error::Error,
{
    let mut last_error = None;
    
    for attempt in 0..policy.max_attempts() {
        if attempt > 0 {
            policy.wait_before_retry(attempt).await;
        }

        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!("Operation succeeded after {} retries", attempt);
                }
                return Ok(result);
            }
            Err(error) => {
                warn!("Operation failed on attempt {}: {}", attempt + 1, error);
                
                if attempt + 1 < policy.max_attempts() && policy.should_retry(attempt + 1, &error) {
                    last_error = Some(error);
                    continue;
                } else {
                    return Err(error);
                }
            }
        }
    }

    // This should never be reached, but just in case
    Err(last_error.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_policy_success_first_try() {
        let policy = RetryPolicy::new(
            3,
            Duration::from_millis(10),
            Duration::from_millis(100),
            true,
        );

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_with_policy(&policy, || {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok::<i32, RetryableError>(42)
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_policy_success_after_retries() {
        let policy = RetryPolicy::new(
            3,
            Duration::from_millis(1),
            Duration::from_millis(10),
            true,
        );

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_with_policy(&policy, || {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err(RetryableError::retryable("temporary failure".to_string()))
                } else {
                    Ok::<i32, RetryableError>(42)
                }
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_policy_max_attempts_reached() {
        let policy = RetryPolicy::new(
            2,
            Duration::from_millis(1),
            Duration::from_millis(10),
            true,
        );

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_with_policy(&policy, || {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Err::<i32, RetryableError>(RetryableError::retryable("always fails".to_string()))
            }
        }).await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_policy_disabled() {
        let policy = RetryPolicy::new(
            3,
            Duration::from_millis(10),
            Duration::from_millis(100),
            false,
        );

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_with_policy(&policy, || {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Err::<i32, RetryableError>(RetryableError::retryable("failure".to_string()))
            }
        }).await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Only one attempt when disabled
    }
}