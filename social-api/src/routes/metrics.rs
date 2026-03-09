use axum::response::IntoResponse;

pub async fn metrics_endpoint() -> impl IntoResponse {
    "# Metrics placeholder\n"
}
