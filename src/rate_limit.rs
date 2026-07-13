use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::error::ApiError;

const LOGIN_WINDOW: Duration = Duration::from_secs(15 * 60);
const MAX_FAILURES: u32 = 5;

#[derive(Clone, Default)]
pub struct LoginRateLimiter {
    attempts: Arc<Mutex<HashMap<String, AttemptWindow>>>,
}

struct AttemptWindow {
    started_at: Instant,
    failures: u32,
}

impl LoginRateLimiter {
    pub fn check(&self, key: &str) -> Result<(), ApiError> {
        let mut attempts = self.attempts.lock().map_err(|_| ApiError::Internal)?;
        prune_expired(&mut attempts);

        if attempts
            .get(key)
            .is_some_and(|attempt| attempt.failures >= MAX_FAILURES)
        {
            return Err(ApiError::RateLimited(
                "too many failed login attempts; try again later".to_owned(),
            ));
        }

        Ok(())
    }

    pub fn record_failure(&self, key: String) -> Result<(), ApiError> {
        let mut attempts = self.attempts.lock().map_err(|_| ApiError::Internal)?;
        prune_expired(&mut attempts);
        let attempt = attempts.entry(key).or_insert(AttemptWindow {
            started_at: Instant::now(),
            failures: 0,
        });
        attempt.failures += 1;
        Ok(())
    }

    pub fn clear(&self, key: &str) -> Result<(), ApiError> {
        self.attempts
            .lock()
            .map_err(|_| ApiError::Internal)?
            .remove(key);
        Ok(())
    }
}

fn prune_expired(attempts: &mut HashMap<String, AttemptWindow>) {
    attempts.retain(|_, attempt| attempt.started_at.elapsed() < LOGIN_WINDOW);
}

#[cfg(test)]
mod tests {
    use super::{LoginRateLimiter, MAX_FAILURES};

    #[test]
    fn limiter_blocks_after_repeated_failures_and_can_be_cleared() {
        let limiter = LoginRateLimiter::default();
        let key = "127.0.0.1:family@demo.invalid";

        for _ in 0..MAX_FAILURES {
            limiter
                .record_failure(key.to_owned())
                .expect("failure should be recorded");
        }
        assert!(limiter.check(key).is_err());

        limiter.clear(key).expect("attempts should clear");
        assert!(limiter.check(key).is_ok());
    }
}
