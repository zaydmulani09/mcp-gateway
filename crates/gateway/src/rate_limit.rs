use std::time::Instant;

use dashmap::DashMap;

use crate::config::RateLimitConfig;

pub enum RateLimitResult {
    Allowed,
    Denied { retry_after_secs: f64 },
}

struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

pub struct RateLimiter {
    buckets: DashMap<String, Bucket>,
    capacity: f64,
    refill_rate: f64,
    enabled: bool,
}

impl RateLimiter {
    pub fn new(config: &RateLimitConfig) -> RateLimiter {
        RateLimiter {
            buckets: DashMap::new(),
            capacity: config.burst,
            refill_rate: config.requests_per_second,
            enabled: config.enabled,
        }
    }

    pub fn check(&self, client_ip: &str, server_name: &str) -> RateLimitResult {
        if !self.enabled {
            return RateLimitResult::Allowed;
        }

        let key = format!("{}:{}", client_ip, server_name);
        let now = Instant::now();
        let capacity = self.capacity;
        let refill_rate = self.refill_rate;

        let mut entry = self.buckets.entry(key).or_insert_with(|| Bucket {
            tokens: capacity,
            last_refill: now,
        });

        let elapsed = now.duration_since(entry.last_refill).as_secs_f64();
        entry.tokens = (entry.tokens + elapsed * refill_rate).min(capacity);
        entry.last_refill = now;

        if entry.tokens >= 1.0 {
            entry.tokens -= 1.0;
            RateLimitResult::Allowed
        } else {
            let retry_after_secs = (1.0 - entry.tokens) / refill_rate;
            RateLimitResult::Denied { retry_after_secs }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RateLimitConfig;
    use std::time::Duration;

    fn make_limiter(rps: f64, burst: f64) -> RateLimiter {
        RateLimiter::new(&RateLimitConfig {
            enabled: true,
            requests_per_second: rps,
            burst,
        })
    }

    #[test]
    fn allows_first_request() {
        let rl = make_limiter(10.0, 20.0);
        assert!(matches!(rl.check("127.0.0.1", "test"), RateLimitResult::Allowed));
    }

    #[test]
    fn denies_after_burst_exhausted() {
        let rl = make_limiter(10.0, 5.0);
        for _ in 0..5 {
            assert!(matches!(rl.check("127.0.0.1", "test"), RateLimitResult::Allowed));
        }
        assert!(matches!(
            rl.check("127.0.0.1", "test"),
            RateLimitResult::Denied { .. }
        ));
    }

    #[tokio::test]
    async fn refills_over_time() {
        let rl = make_limiter(10.0, 5.0);
        for _ in 0..5 {
            rl.check("127.0.0.1", "test");
        }
        assert!(matches!(
            rl.check("127.0.0.1", "test"),
            RateLimitResult::Denied { .. }
        ));
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(matches!(rl.check("127.0.0.1", "test"), RateLimitResult::Allowed));
    }
}
