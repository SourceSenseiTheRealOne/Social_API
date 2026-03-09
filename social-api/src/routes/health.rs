use axum::Json;
use serde_json::{json, Value};

pub async fn live() -> Json<Value> {
    Json(json!({ "status": "alive" }))
}

pub async fn ready() -> Json<Value> {
    Json(json!({ "status": "ready" }))
}
