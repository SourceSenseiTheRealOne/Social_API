use axum::extract::State;
use std::sync::Arc;

use crate::routes::health::InfraState;

pub async fn metrics_endpoint(State(state): State<Arc<InfraState>>) -> String {
    state.metrics.render()
}
