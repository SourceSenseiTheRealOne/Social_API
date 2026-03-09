use axum::{extract::Request, extract::State, middleware::Next, response::Response};
use std::sync::Arc;
use std::time::Instant;

use crate::observability::metrics::{AppMetrics, HttpDurationLabels, HttpLabels};

pub async fn metrics_middleware(
    State(metrics): State<Arc<AppMetrics>>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let start = Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed();
    let status = response.status().as_u16();

    metrics
        .http_requests_total
        .get_or_create(&HttpLabels {
            method: method.clone(),
            path: path.clone(),
            status,
        })
        .inc();

    metrics
        .http_request_duration
        .get_or_create(&HttpDurationLabels { method, path })
        .observe(duration.as_secs_f64());

    response
}
