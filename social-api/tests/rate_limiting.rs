mod common;

use serde_json::json;

use axum::Router;
use social_api::cache::rate_limiter::{RateLimitInfo, RateLimiter};
use social_api::cache::LikeCache;
use social_api::errors::AppError;
use social_api::models::like::{LikeEvent, UserInfo};
use social_api::observability::AppMetrics;
use social_api::repositories::{LikeRepository, PgLikeRepository};
use social_api::routes::health::{HealthState, InfraState};
use social_api::routes::likes::AppState;
use social_api::services::LikeService;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use axum::{
    middleware::{from_fn, from_fn_with_state},
    routing::{delete, get, post},
};

use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;

/// A rate limiter that actually counts calls and enforces limits.
struct CountingRateLimiter {
    counts: std::sync::Mutex<HashMap<String, u32>>,
}

impl CountingRateLimiter {
    fn new() -> Self {
        Self {
            counts: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl RateLimiter for CountingRateLimiter {
    async fn check_rate_limit(&self, _scope: &str, identifier: &str, limit: u32) -> RateLimitInfo {
        let mut counts = self.counts.lock().unwrap();
        let count = counts.entry(identifier.to_string()).or_insert(0);
        *count += 1;

        let exceeded = *count > limit;
        let remaining = if exceeded { 0 } else { limit - *count };

        RateLimitInfo {
            limit,
            remaining,
            reset_at: 60,
            exceeded,
        }
    }
}

struct MockCache {
    counts: std::sync::Mutex<HashMap<(String, Uuid), i64>>,
}

impl MockCache {
    fn new() -> Self {
        Self {
            counts: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl LikeCache for MockCache {
    async fn get_count(&self, content_type: &str, content_id: Uuid) -> Option<i64> {
        let counts = self.counts.lock().unwrap();
        counts.get(&(content_type.to_string(), content_id)).copied()
    }

    async fn set_count(&self, content_type: &str, content_id: Uuid, count: i64) {
        let mut counts = self.counts.lock().unwrap();
        counts.insert((content_type.to_string(), content_id), count);
    }

    async fn increment_count(&self, content_type: &str, content_id: Uuid) -> Option<i64> {
        let mut counts = self.counts.lock().unwrap();
        let key = (content_type.to_string(), content_id);
        let count = counts.entry(key).or_insert(0);
        *count += 1;
        Some(*count)
    }

    async fn decrement_count(&self, content_type: &str, content_id: Uuid) -> Option<i64> {
        let mut counts = self.counts.lock().unwrap();
        let key = (content_type.to_string(), content_id);
        let count = counts.entry(key).or_insert(0);
        *count = (*count - 1).max(0);
        Some(*count)
    }

    async fn get_content_validation(&self, _content_type: &str, _content_id: Uuid) -> Option<bool> {
        None
    }

    async fn set_content_validation(&self, _content_type: &str, _content_id: Uuid, _valid: bool) {}

    async fn is_available(&self) -> bool {
        true
    }

    async fn acquire_stampede_lock(&self, _key: &str, _ttl: Duration) -> bool {
        true
    }
}

struct MockContentClient;

#[async_trait]
impl social_api::clients::ContentClient for MockContentClient {
    async fn validate_content(
        &self,
        _content_type: &str,
        _content_id: Uuid,
    ) -> Result<bool, AppError> {
        Ok(true)
    }
}

struct MockProfileClient;

#[async_trait]
impl social_api::clients::ProfileClient for MockProfileClient {
    async fn validate_token(&self, token: &str) -> Result<UserInfo, AppError> {
        if !token.starts_with("tok_user_") {
            return Err(AppError::Unauthorized);
        }
        let suffix = &token[9..];
        let user_id = format!("550e8400-e29b-41d4-a716-44665544000{}", suffix)
            .parse()
            .map_err(|_| AppError::Unauthorized)?;
        Ok(UserInfo { user_id })
    }
}

async fn setup_rate_limited_app(write_limit: u32) -> (Router, PgPool, String) {
    let schema = format!("test_{}", Uuid::new_v4().to_string().replace("-", ""));

    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://social:social@localhost:5432/social_api".to_string());

    let pool = PgPool::connect(&database_url).await.unwrap();

    sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", schema))
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(&format!("SET search_path TO \"{}\"", schema))
        .execute(&pool)
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let metrics = AppMetrics::new();
    let like_cache: Arc<dyn LikeCache> = Arc::new(MockCache::new());
    let rate_limiter: Arc<dyn RateLimiter> = Arc::new(CountingRateLimiter::new());
    let content_client: Arc<dyn social_api::clients::ContentClient> = Arc::new(MockContentClient);
    let profile_client: Arc<dyn social_api::clients::ProfileClient> = Arc::new(MockProfileClient);
    let repo: Arc<dyn LikeRepository> = Arc::new(PgLikeRepository::new(pool.clone(), pool.clone()));

    let (event_tx, _) = broadcast::channel::<LikeEvent>(1024);

    let like_service = Arc::new(LikeService::new(
        repo,
        like_cache.clone(),
        content_client,
        event_tx.clone(),
    ));

    let app_state = Arc::new(AppState {
        like_service: like_service.clone(),
    });

    let auth_state = Arc::new(social_api::middleware::auth::AuthState { profile_client });

    let rate_limit_state = Arc::new(social_api::middleware::rate_limit::RateLimitState {
        rate_limiter,
        write_limit,
        read_limit: 1000,
    });

    let health_state = Arc::new(HealthState {
        db_pool: pool.clone(),
        cache: like_cache,
    });

    let infra_state = Arc::new(InfraState {
        health: health_state,
        metrics: metrics.clone(),
    });

    let auth_routes = Router::new()
        .route("/v1/likes", post(social_api::routes::likes::create_like))
        .route(
            "/v1/likes/{content_type}/{content_id}",
            delete(social_api::routes::likes::unlike),
        )
        .route(
            "/v1/likes/{content_type}/{content_id}/status",
            get(social_api::routes::likes::get_status),
        )
        .route(
            "/v1/likes/user",
            get(social_api::routes::likes::get_user_likes),
        )
        .route(
            "/v1/likes/batch/statuses",
            post(social_api::routes::likes::batch_statuses),
        )
        .layer(from_fn_with_state(
            rate_limit_state.clone(),
            social_api::middleware::rate_limit::write_rate_limit_middleware,
        ))
        .layer(from_fn_with_state(
            auth_state,
            social_api::middleware::auth::auth_middleware,
        ))
        .with_state(app_state.clone());

    let public_routes = Router::new()
        .route(
            "/v1/likes/{content_type}/{content_id}/count",
            get(social_api::routes::likes::get_count),
        )
        .route(
            "/v1/likes/batch/counts",
            post(social_api::routes::likes::batch_counts),
        )
        .route("/v1/likes/top", get(social_api::routes::likes::get_top))
        .layer(from_fn_with_state(
            rate_limit_state,
            social_api::middleware::rate_limit::read_rate_limit_middleware,
        ))
        .with_state(app_state);

    let infra_routes = Router::new()
        .route("/health/live", get(social_api::routes::health::live))
        .route("/health/ready", get(social_api::routes::health::ready))
        .route(
            "/metrics",
            get(social_api::routes::metrics::metrics_endpoint),
        )
        .with_state(infra_state);

    let app = Router::new()
        .merge(auth_routes)
        .merge(public_routes)
        .merge(infra_routes)
        .layer(from_fn(
            social_api::middleware::request_id::request_id_middleware,
        ))
        .layer(from_fn_with_state(
            metrics,
            social_api::middleware::metrics::metrics_middleware,
        ));

    (app, pool, schema)
}

#[tokio::test]
async fn write_rate_limit_triggers_at_31() {
    let (app, pool, schema) = setup_rate_limited_app(30).await;

    // Send 30 requests (at the limit)
    for i in 0..30 {
        let content_id = Uuid::new_v4().to_string();
        let (status, _) = common::request(
            &app,
            "POST",
            "/v1/likes",
            Some(json!({"content_type": "post", "content_id": content_id})),
            Some("tok_user_1"),
        )
        .await;
        // Should not be rate limited yet
        assert_ne!(status, 429, "Rate limited too early at request {}", i + 1);
    }

    // 31st request should be rate limited
    let (status, body) = common::request(
        &app,
        "POST",
        "/v1/likes",
        Some(json!({"content_type": "post", "content_id": Uuid::new_v4().to_string()})),
        Some("tok_user_1"),
    )
    .await;

    assert_eq!(status, 429);
    assert_eq!(body["error"]["code"], "RATE_LIMITED");

    common::teardown(&pool, &schema).await;
}
