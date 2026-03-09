use axum::extract::State;
use std::sync::Arc;

use crate::observability::AppMetrics;

pub async fn metrics_endpoint(
    State(metrics): State<Arc<AppMetrics>>,
) -> String {
    metrics.render()
}
