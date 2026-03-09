use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;

pub async fn metrics_middleware(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let start = Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed();

    let status = response.status().as_u16();

    tracing::info!(
        method = %method,
        path = %path,
        status = status,
        duration_ms = duration.as_millis() as u64,
        "Request completed"
    );

    response
}
