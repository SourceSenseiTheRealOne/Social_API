use async_trait::async_trait;
use deadpool_redis::Pool;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u32,
    pub remaining: u32,
    pub reset_at: u64,
    pub exceeded: bool,
}

#[async_trait]
pub trait RateLimiter: Send + Sync {
    async fn check_rate_limit(&self, scope: &str, identifier: &str, limit: u32) -> RateLimitInfo;
}

pub struct RedisRateLimiter {
    pool: Pool,
    window_secs: u64,
}

impl RedisRateLimiter {
    pub fn new(pool: Pool, window_secs: u64) -> Self {
        Self { pool, window_secs }
    }
}

#[async_trait]
impl RateLimiter for RedisRateLimiter {
    async fn check_rate_limit(&self, scope: &str, identifier: &str, limit: u32) -> RateLimitInfo {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let window = now / self.window_secs;
        let reset_at = (window + 1) * self.window_secs;

        let key = format!("ratelimit:{scope}:{identifier}:{window}");

        let Ok(mut conn) = self.pool.get().await else {
            return RateLimitInfo {
                limit,
                remaining: limit,
                reset_at,
                exceeded: false,
            };
        };

        let script = redis::Script::new(
            r#"
            local current = redis.call('INCR', KEYS[1])
            if current == 1 then
                redis.call('EXPIRE', KEYS[1], ARGV[1])
            end
            return current
            "#,
        );

        let count: u32 = match script
            .key(&key)
            .arg(self.window_secs * 2)
            .invoke_async(&mut *conn)
            .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "Rate limit check failed, allowing through");
                return RateLimitInfo {
                    limit,
                    remaining: limit,
                    reset_at,
                    exceeded: false,
                };
            }
        };

        let exceeded = count > limit;
        let remaining = if exceeded { 0 } else { limit - count };

        RateLimitInfo {
            limit,
            remaining,
            reset_at,
            exceeded,
        }
    }
}
