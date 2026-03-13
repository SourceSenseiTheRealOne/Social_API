mod cache;
mod circuit_breaker;
mod clients;
mod config;
mod errors;
mod middleware;
mod models;
mod observability;
mod repositories;
mod routes;
mod services;
mod sse;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    middleware::from_fn,
    middleware::from_fn_with_state,
    routing::{delete, get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::broadcast;

use cache::like_cache::RedisLikeCache;
use cache::rate_limiter::RedisRateLimiter;
use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use clients::content_client::HttpContentClient;
use clients::profile_client::HttpProfileClient;
use config::Config;
use middleware::auth::{auth_middleware, AuthState};
use middleware::metrics::metrics_middleware;
use middleware::rate_limit::{
    read_rate_limit_middleware, write_rate_limit_middleware, RateLimitState,
};
use middleware::request_id::request_id_middleware;
use observability::{init_logging, AppMetrics};
use repositories::PgLikeRepository;
use routes::health::{HealthState, InfraState};
use routes::likes::AppState;
use services::LikeService;

#[tokio::main]
async fn main() {
    init_logging();

    let config = Config::from_env();
    let metrics = AppMetrics::new();

    let write_pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to write database");

    let read_pool = PgPoolOptions::new()
        .max_connections(config.db_read_max_connections)
        .connect(&config.read_database_url)
        .await
        .expect("Failed to connect to read database");

    sqlx::migrate!()
        .run(&write_pool)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database migrations applied");

    let redis_cfg = deadpool_redis::Config::from_url(&config.redis_url);
    let redis_pool = redis_cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Failed to create Redis pool");

    let like_cache: Arc<dyn cache::LikeCache> = Arc::new(RedisLikeCache::new(
        redis_pool.clone(),
        config.cache_ttl_count,
        config.cache_ttl_content_validation,
    ));

    let rate_limiter: Arc<dyn cache::rate_limiter::RateLimiter> =
        Arc::new(RedisRateLimiter::new(redis_pool, 60));

    let cb_config = CircuitBreakerConfig {
        failure_threshold: config.cb_failure_threshold,
        failure_rate_threshold: config.cb_failure_rate_threshold,
        failure_rate_window: std::time::Duration::from_secs(30),
        open_duration: std::time::Duration::from_secs(config.cb_open_duration_secs),
        half_open_successes: config.cb_half_open_successes,
    };

    let profile_cb = Arc::new(CircuitBreaker::new(
        "profile-api".to_string(),
        cb_config.clone(),
    ));

    let mut content_cbs = HashMap::new();
    for content_type in config.content_api_urls.keys() {
        content_cbs.insert(
            content_type.clone(),
            Arc::new(CircuitBreaker::new(
                format!("content-api-{}", content_type),
                cb_config.clone(),
            )),
        );
    }

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();

    let profile_client: Arc<dyn clients::ProfileClient> = Arc::new(HttpProfileClient::new(
        http_client.clone(),
        config.profile_api_url.clone(),
        profile_cb,
    ));

    let content_client: Arc<dyn clients::ContentClient> = Arc::new(HttpContentClient::new(
        http_client,
        config.content_api_urls.clone(),
        content_cbs,
        like_cache.clone(),
    ));

    let repo: Arc<dyn repositories::LikeRepository> =
        Arc::new(PgLikeRepository::new(write_pool.clone(), read_pool));

    let (event_tx, _) = broadcast::channel::<models::like::LikeEvent>(1024);

    let like_service = Arc::new(LikeService::new(
        repo.clone(),
        like_cache.clone(),
        content_client,
        event_tx.clone(),
    ));

    // Warm caches before accepting traffic
    cache::warming::warm_caches(&repo, &like_cache).await;

    let app_state = Arc::new(AppState {
        like_service: like_service.clone(),
    });

    let auth_state = Arc::new(AuthState { profile_client });

    let rate_limit_state = Arc::new(RateLimitState {
        rate_limiter,
        write_limit: config.rate_limit_write,
        read_limit: config.rate_limit_read,
    });

    let health_state = Arc::new(HealthState {
        db_pool: write_pool,
        cache: like_cache,
    });

    let auth_routes = Router::new()
        .route("/v1/likes", post(routes::likes::create_like))
        .route(
            "/v1/likes/{content_type}/{content_id}",
            delete(routes::likes::unlike),
        )
        .route(
            "/v1/likes/{content_type}/{content_id}/status",
            get(routes::likes::get_status),
        )
        .route("/v1/likes/user", get(routes::likes::get_user_likes))
        .route(
            "/v1/likes/batch/statuses",
            post(routes::likes::batch_statuses),
        )
        .layer(from_fn_with_state(
            rate_limit_state.clone(),
            write_rate_limit_middleware,
        ))
        .layer(from_fn_with_state(auth_state, auth_middleware))
        .with_state(app_state.clone());

    let public_routes = Router::new()
        .route(
            "/v1/likes/{content_type}/{content_id}/count",
            get(routes::likes::get_count),
        )
        .route("/v1/likes/batch/counts", post(routes::likes::batch_counts))
        .route("/v1/likes/top", get(routes::likes::get_top))
        .layer(from_fn_with_state(
            rate_limit_state,
            read_rate_limit_middleware,
        ))
        .with_state(app_state);

    let sse_event_tx = event_tx.clone();
    let sse_route = Router::new().route(
        "/v1/likes/stream",
        get(move || {
            let rx = sse_event_tx.subscribe();
            sse::sse_stream(rx)
        }),
    );

    let infra_state = Arc::new(InfraState {
        health: health_state,
        metrics: metrics.clone(),
    });

    let infra_routes = Router::new()
        .route("/health/live", get(routes::health::live))
        .route("/health/ready", get(routes::health::ready))
        .route("/metrics", get(routes::metrics::metrics_endpoint))
        .with_state(infra_state);

    let app = Router::new()
        .merge(auth_routes)
        .merge(public_routes)
        .merge(sse_route)
        .merge(infra_routes)
        .layer(from_fn(request_id_middleware))
        .layer(from_fn_with_state(metrics.clone(), metrics_middleware));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.http_port));
    tracing::info!("Starting Social API on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();

    tracing::info!("Server shut down gracefully");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received Ctrl+C, shutting down"),
        _ = terminate => tracing::info!("Received SIGTERM, shutting down"),
    }
}
