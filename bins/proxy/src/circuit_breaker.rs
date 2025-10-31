use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Circuit is open, failing fast
    HalfOpen, // Testing if service has recovered
}

#[derive(Debug)]
pub struct CircuitBreakerInner {
    state: CircuitState,
    failure_count: u64,
    success_count: u64,
    last_failure_time: Option<Instant>,
    failure_threshold: u64,
    recovery_timeout: Duration,
    half_open_max_calls: u64,
}

impl CircuitBreakerInner {
    pub fn new(failure_threshold: u64, recovery_timeout: Duration, half_open_max_calls: u64) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            failure_threshold,
            recovery_timeout,
            half_open_max_calls,
        }
    }

    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed() >= self.recovery_timeout {
                        info!("Circuit breaker transitioning to half-open state");
                        self.state = CircuitState::HalfOpen;
                        self.success_count = 0;
                        true
                    } else {
                        debug!("Circuit breaker is open, rejecting request");
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                self.success_count < self.half_open_max_calls
            }
        }
    }

    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.half_open_max_calls {
                    info!("Circuit breaker closing after successful recovery");
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    self.last_failure_time = None;
                }
            }
            CircuitState::Open => {
                // Should not happen, but reset if it does
                self.state = CircuitState::Closed;
                self.failure_count = 0;
                self.success_count = 0;
                self.last_failure_time = None;
            }
        }
        debug!("Circuit breaker recorded success, state: {:?}", self.state);
    }

    pub fn record_failure(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold {
                    warn!("Circuit breaker opening due to {} failures", self.failure_count);
                    self.state = CircuitState::Open;
                    self.last_failure_time = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                warn!("Circuit breaker opening again after failure in half-open state");
                self.state = CircuitState::Open;
                self.failure_count += 1;
                self.last_failure_time = Some(Instant::now());
                self.success_count = 0;
            }
            CircuitState::Open => {
                self.failure_count += 1;
                self.last_failure_time = Some(Instant::now());
            }
        }
        debug!("Circuit breaker recorded failure, state: {:?}, count: {}", self.state, self.failure_count);
    }

    pub fn get_state(&self) -> CircuitState {
        self.state.clone()
    }
}

#[derive(Clone)]
pub struct CircuitBreaker {
    inner: Arc<Mutex<CircuitBreakerInner>>,
    enabled: bool,
}

impl CircuitBreaker {
    pub fn new(
        failure_threshold: u64,
        recovery_timeout: Duration,
        half_open_max_calls: u64,
        enabled: bool,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CircuitBreakerInner::new(
                failure_threshold,
                recovery_timeout,
                half_open_max_calls,
            ))),
            enabled,
        }
    }

    pub async fn can_execute(&self) -> bool {
        if !self.enabled {
            return true;
        }

        let mut inner = self.inner.lock().await;
        inner.can_execute()
    }

    pub async fn record_success(&self) {
        if !self.enabled {
            return;
        }

        let mut inner = self.inner.lock().await;
        inner.record_success();
    }

    pub async fn record_failure(&self) {
        if !self.enabled {
            return;
        }

        let mut inner = self.inner.lock().await;
        inner.record_failure();
    }

    pub async fn get_state(&self) -> CircuitState {
        if !self.enabled {
            return CircuitState::Closed;
        }

        let inner = self.inner.lock().await;
        inner.get_state()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_circuit_breaker_closed_to_open() {
        let cb = CircuitBreaker::new(3, Duration::from_millis(100), 2, true);
        
        // Should be closed initially
        assert!(cb.can_execute().await);
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        
        // Record failures
        cb.record_failure().await;
        cb.record_failure().await;
        assert!(cb.can_execute().await); // Still closed
        
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);
        assert!(!cb.can_execute().await); // Now open
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(50), 1, true);
        
        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);
        
        // Wait for recovery timeout
        sleep(Duration::from_millis(60)).await;
        
        // Should transition to half-open
        assert!(cb.can_execute().await);
        assert_eq!(cb.get_state().await, CircuitState::HalfOpen);
        
        // Record success to close
        cb.record_success().await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_disabled() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(100), 1, false);
        
        // Should always allow when disabled
        for _ in 0..10 {
            cb.record_failure().await;
            assert!(cb.can_execute().await);
        }
        
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }
}