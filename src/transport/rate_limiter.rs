use governor::{Quota, RateLimiter as GovRateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Token-bucket rate limiter wrapping the `governor` crate.
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<GovRateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
}

impl RateLimiter {
    /// Create a rate limiter that allows `per_second` requests per second.
    pub fn new(per_second: u32) -> Self {
        let per_second = per_second.max(1);
        let quota = Quota::per_second(NonZeroU32::new(per_second).unwrap());
        let limiter = GovRateLimiter::direct(quota);
        Self {
            inner: Arc::new(limiter),
        }
    }

    /// Wait until a request is allowed.
    pub async fn wait(&self) {
        self.inner.until_ready().await;
    }
}
