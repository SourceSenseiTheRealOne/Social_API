use async_trait::async_trait;
use deadpool_redis::{Connection, Pool};
use redis::AsyncCommands;
use std::time::Duration;
use uuid::Uuid;

#[async_trait]
pub trait LikeCache: Send + Sync {
    async fn get_count(&self, content_type: &str, content_id: Uuid) -> Option<i64>;
    async fn set_count(&self, content_type: &str, content_id: Uuid, count: i64);
    async fn increment_count(&self, content_type: &str, content_id: Uuid) -> Option<i64>;
    async fn decrement_count(&self, content_type: &str, content_id: Uuid) -> Option<i64>;
    async fn get_content_validation(&self, content_type: &str, content_id: Uuid) -> Option<bool>;
    async fn set_content_validation(&self, content_type: &str, content_id: Uuid, valid: bool);
    async fn is_available(&self) -> bool;
    async fn acquire_stampede_lock(&self, key: &str, ttl: Duration) -> bool;
}

pub struct RedisLikeCache {
    pool: Pool,
    count_ttl: Duration,
    validation_ttl: Duration,
}

impl RedisLikeCache {
    pub fn new(pool: Pool, count_ttl_secs: u64, validation_ttl_secs: u64) -> Self {
        Self {
            pool,
            count_ttl: Duration::from_secs(count_ttl_secs),
            validation_ttl: Duration::from_secs(validation_ttl_secs),
        }
    }

    async fn get_conn(&self) -> Option<Connection> {
        match self.pool.get().await {
            Ok(conn) => Some(conn),
            Err(e) => {
                tracing::warn!(error = %e, "Redis unavailable, operating in degraded mode");
                None
            }
        }
    }

    fn count_key(content_type: &str, content_id: Uuid) -> String {
        format!("likes:count:{content_type}:{content_id}")
    }

    fn validation_key(content_type: &str, content_id: Uuid) -> String {
        format!("likes:valid:{content_type}:{content_id}")
    }

    fn stampede_lock_key(key: &str) -> String {
        format!("likes:lock:{key}")
    }
}

#[async_trait]
impl LikeCache for RedisLikeCache {
    async fn get_count(&self, content_type: &str, content_id: Uuid) -> Option<i64> {
        let mut conn = self.get_conn().await?;
        let key = Self::count_key(content_type, content_id);
        conn.get(&key).await.ok()
    }

    async fn set_count(&self, content_type: &str, content_id: Uuid, count: i64) {
        let Some(mut conn) = self.get_conn().await else {
            return;
        };
        let key = Self::count_key(content_type, content_id);
        let _: Result<(), _> = conn.set_ex(&key, count, self.count_ttl.as_secs()).await;
    }

    async fn increment_count(&self, content_type: &str, content_id: Uuid) -> Option<i64> {
        let mut conn = self.get_conn().await?;
        let key = Self::count_key(content_type, content_id);
        conn.incr(&key, 1).await.ok()
    }

    async fn decrement_count(&self, content_type: &str, content_id: Uuid) -> Option<i64> {
        let mut conn = self.get_conn().await?;
        let key = Self::count_key(content_type, content_id);

        let script = redis::Script::new(
            r#"
            local current = redis.call('GET', KEYS[1])
            if current == false then return nil end
            local new_val = math.max(tonumber(current) - 1, 0)
            redis.call('SET', KEYS[1], new_val)
            return new_val
            "#,
        );
        script.key(key).invoke_async(&mut conn).await.ok()
    }

    async fn get_content_validation(&self, content_type: &str, content_id: Uuid) -> Option<bool> {
        let mut conn = self.get_conn().await?;
        let key = Self::validation_key(content_type, content_id);
        let val: String = conn.get(&key).await.ok()?;
        Some(val == "1")
    }

    async fn set_content_validation(&self, content_type: &str, content_id: Uuid, valid: bool) {
        let Some(mut conn) = self.get_conn().await else {
            return;
        };
        let key = Self::validation_key(content_type, content_id);
        let val = if valid { "1" } else { "0" };
        let _: Result<(), _> = conn.set_ex(&key, val, self.validation_ttl.as_secs()).await;
    }

    async fn is_available(&self) -> bool {
        self.get_conn().await.is_some()
    }

    async fn acquire_stampede_lock(&self, key: &str, ttl: Duration) -> bool {
        let Some(mut conn) = self.get_conn().await else {
            return false;
        };
        let lock_key = Self::stampede_lock_key(key);
        let result: bool = conn.set_nx(&lock_key, "1").await.unwrap_or(false);
        if result {
            let _: Result<(), _> = conn.expire(&lock_key, ttl.as_millis() as i64).await;
        }
        result
    }
}
