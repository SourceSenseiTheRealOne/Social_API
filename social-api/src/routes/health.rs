use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::cache::LikeCache;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checks: Option<HealthChecks>,
}

#[derive(Serialize)]
pub struct HealthChecks {
    pub database: CheckResult,
    pub redis: CheckResult,
}

#[derive(Serialize)]
pub struct CheckResult {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub struct HealthState {
    pub db_pool: sqlx::PgPool,
    pub cache: Arc<dyn LikeCache>,
}

pub async fn live() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "alive".to_string(),
        checks: None,
    })
}

pub async fn ready(
    State(state): State<Arc<HealthState>>,
) -> Json<HealthResponse> {
    let db_check = match sqlx::query("SELECT 1")
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(_) => CheckResult {
            status: "ok".to_string(),
            message: None,
        },
        Err(e) => CheckResult {
            status: "error".to_string(),
            message: Some(e.to_string()),
        },
    };

    let redis_check = if state.cache.is_available().await {
        CheckResult {
            status: "ok".to_string(),
            message: None,
        }
    } else {
        CheckResult {
            status: "error".to_string(),
            message: Some("Redis unavailable".to_string()),
        }
    };

    let all_ok = db_check.status == "ok" && redis_check.status == "ok";

    Json(HealthResponse {
        status: if all_ok { "ready" } else { "degraded" }.to_string(),
        checks: Some(HealthChecks {
            database: db_check,
            redis: redis_check,
        }),
    })
}
