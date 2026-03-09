mod cache;
mod circuit_breaker;
mod clients;
mod config;
mod errors;
mod models;
mod repositories;
mod services;

use axum::{routing::get, Json, Router};
use config::Config;
use serde_json::{json, Value};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        . with_env_filter(
            EnvFilter::from_default_env().add_directive("social_api=info".parse().unwrap()),
        )
        .json()
        .init();

    let config = Config::from_env();

    let app = Router::new().route("/health/live", get(health_live));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.http_port));
    tracing::info!("Starting Social API on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_live() -> Json<Value> {
    Json(json!({ "status": "alive" }))
}
