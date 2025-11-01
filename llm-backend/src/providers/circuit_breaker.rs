use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing, reject requests
    HalfOpen, // Testing if service recovered
}

/// Circuit breaker for provider fault tolerance
pub struct CircuitBreaker {
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: AtomicU64, // Unix timestamp in seconds
    threshold: u32,
    cooldown: Duration,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, cooldown: Duration) -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            threshold,
            cooldown,
        }
    }

    /// Check if the circuit is open (failing)
    pub fn is_open(&self) -> bool {
        self.state() == CircuitState::Open
    }

    /// Get the current circuit state
    pub fn state(&self) -> CircuitState {
        let failures = self.failure_count.load(Ordering::Relaxed);

        if failures < self.threshold {
            return CircuitState::Closed;
        }

        // Circuit is open, check if cooldown has elapsed
        let last_failure = self.last_failure_time.load(Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now - last_failure >= self.cooldown.as_secs() {
            CircuitState::HalfOpen
        } else {
            CircuitState::Open
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        self.success_count.fetch_add(1, Ordering::Relaxed);

        // If we're in half-open state and got a success, reset
        if self.state() == CircuitState::HalfOpen {
            self.reset();
        }
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_failure_time.store(now, Ordering::Relaxed);
    }

    /// Reset the circuit breaker
    pub fn reset(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
    }

    /// Get failure count
    pub fn failures(&self) -> u32 {
        self.failure_count.load(Ordering::Relaxed)
    }

    /// Get success count
    #[allow(dead_code)]
    pub fn successes(&self) -> u32 {
        self.success_count.load(Ordering::Relaxed)
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(5, Duration::from_secs(60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_circuit_closed_initially() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1));
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(!cb.is_open());
    }

    #[test]
    fn test_circuit_opens_after_threshold() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1));

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.is_open());
    }

    #[test]
    fn test_circuit_half_open_after_cooldown() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(100));

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for cooldown
        thread::sleep(Duration::from_millis(150));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_resets_on_success_in_half_open() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(100));

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for cooldown
        thread::sleep(Duration::from_millis(150));
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Success should reset
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failures(), 0);
    }

    #[test]
    fn test_manual_reset() {
        let cb = CircuitBreaker::new(2, Duration::from_secs(1));

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failures(), 0);
    }
}
