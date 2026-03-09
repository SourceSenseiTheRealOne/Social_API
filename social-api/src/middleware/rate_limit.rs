use axum::{
    extract::{ConnectInfo, Request, State},
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::cache::rate_limiter::{RateLimitInfo, RateLimiter};
use crate::errors::AppError;
use crate::models::like::UserInfo;

pub struct RateLimitState {
    pub rate_limiter: Arc<dyn RateLimiter>,
    pub write_limit: u32,
    pub read_limit: u32,
}

pub async fn write_rate_limit_middleware(
    State(state): State<Arc<RateLimitState>>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user_info = request
        .extensions()
        .get::<UserInfo>()
        .ok_or(AppError::Unauthorized)?;

    let info = state
        .rate_limiter
        .check_rate_limit("write", &user_info.user_id.to_string(), state.write_limit)
        .await;

    if info.exceeded {
        let retry_after = info.reset_at.saturating_sub(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        return Err(AppError::RateLimited {
            retry_after: retry_after.max(1),
        });
    }

    let mut response = next.run(request).await;
    add_rate_limit_headers(&mut response, &info);

    Ok(response)
}

pub async fn read_rate_limit_middleware(
    State(state): State<Arc<RateLimitState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let ip = addr.ip().to_string();

    let info = state
        .rate_limiter
        .check_rate_limit("read", &ip, state.read_limit)
        .await;

    if info.exceeded {
        let retry_after = info.reset_at.saturating_sub(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        return Err(AppError::RateLimited {
            retry_after: retry_after.max(1),
        });
    }

    let mut response = next.run(request).await;
    add_rate_limit_headers(&mut response, &info);

    Ok(response)
}

fn add_rate_limit_headers(response: &mut Response, info: &RateLimitInfo) {
    let headers = response.headers_mut();

    if let Ok(v) = HeaderValue::from_str(&info.limit.to_string()) {
        headers.insert("X-RateLimit-Limit", v);
    }
    if let Ok(v) = HeaderValue::from_str(&info.remaining.to_string()) {
        headers.insert("X-RateLimit-Remaining", v);
    }
    if let Ok(v) = HeaderValue::from_str(&info.reset_at.to_string()) {
        headers.insert("X-RateLimit-Reset", v);
    }
}
