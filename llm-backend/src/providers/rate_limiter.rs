use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

/// Rate limiter wrapper for provider requests
pub struct RateLimiter {
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl RateLimiter {
    /// Create a new rate limiter with requests per minute
    pub fn new(requests_per_minute: u32) -> Self {
        let quota = Quota::per_minute(
            NonZeroU32::new(requests_per_minute).unwrap_or(NonZeroU32::new(60).unwrap()),
        );
        let limiter = Arc::new(GovernorRateLimiter::direct(quota));

        Self { limiter }
    }

    /// Check if a request is allowed (non-blocking)
    #[allow(dead_code)]
    pub fn check(&self) -> bool {
        self.limiter.check().is_ok()
    }

    /// Wait until a request is allowed (blocking)
    pub async fn until_ready(&self) {
        while self.limiter.check().is_err() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(60) // Default 60 requests per minute
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_requests() {
        let limiter = RateLimiter::new(10);

        // First request should be allowed
        assert!(limiter.check());
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_after_limit() {
        let limiter = RateLimiter::new(2);

        // First two requests should pass
        assert!(limiter.check());
        assert!(limiter.check());

        // Third should fail (exceeded quota)
        assert!(!limiter.check());
    }
}
