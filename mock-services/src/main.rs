use axum::{
    Router,
    routing::get,
    extract::Path,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    println!("Starting mock services...");

    let valid_content = Arc::new(build_valid_content());
    let valid_tokens = Arc::new(build_valid_tokens());

    let post_api = serve_content_api("post", 8081, valid_content.clone());
    let bonus_hunter_api = serve_content_api("bonus_hunter", 8082, valid_content.clone());
    let top_picks_api = serve_content_api("top_picks", 8083, valid_content.clone());
    let profile_api = serve_profile_api(8084, valid_tokens);

    tokio::join!(post_api, bonus_hunter_api, top_picks_api, profile_api);
}

fn build_valid_content() -> HashMap<String, HashSet<Uuid>> {
    let mut map = HashMap::new();

    let post_ids: HashSet<Uuid> = [
        "731b0395-4888-4822-b516-05b4b7bf2089",
        "a1b2c3d4-e5f6-4321-abcd-111111111111",
        "a1b2c3d4-e5f6-4321-abcd-222222222222",
        "a1b2c3d4-e5f6-4321-abcd-333333333333",
        "a1b2c3d4-e5f6-4321-abcd-444444444444",
        "a1b2c3d4-e5f6-4321-abcd-555555555555",
    ]
    .iter()
    .map(|s| s.parse().unwrap())
    .collect();

    let bonus_hunter_ids: HashSet<Uuid> = [
        "b1b2c3d4-e5f6-4321-abcd-111111111111",
        "b1b2c3d4-e5f6-4321-abcd-222222222222",
        "b1b2c3d4-e5f6-4321-abcd-333333333333",
        "b1b2c3d4-e5f6-4321-abcd-444444444444",
        "b1b2c3d4-e5f6-4321-abcd-555555555555",
    ]
    .iter()
    .map(|s| s.parse().unwrap())
    .collect();

    let top_picks_ids: HashSet<Uuid> = [
        "c1b2c3d4-e5f6-4321-abcd-111111111111",
        "c1b2c3d4-e5f6-4321-abcd-222222222222",
        "c1b2c3d4-e5f6-4321-abcd-333333333333",
        "c1b2c3d4-e5f6-4321-abcd-444444444444",
        "c1b2c3d4-e5f6-4321-abcd-555555555555",
    ]
    .iter()
    .map(|s| s.parse().unwrap())
    .collect();

    map.insert("post".to_string(), post_ids);
    map.insert("bonus_hunter".to_string(), bonus_hunter_ids);
    map.insert("top_picks".to_string(), top_picks_ids);

    map
}

fn build_valid_tokens() -> HashMap<String, Uuid> {
    let mut map = HashMap::new();
    for i in 1..=5 {
        map.insert(
            format!("tok_user_{}", i),
            format!("550e8400-e29b-41d4-a716-44665544000{}", i)
                .parse()
                .unwrap(),
        );
    }
    map
}

#[derive(Serialize)]
struct ContentResponse {
    id: Uuid,
    title: String,
    content_type: String,
}

async fn serve_content_api(
    content_type: &'static str,
    port: u16,
    valid_content: Arc<HashMap<String, HashSet<Uuid>>>,
) {
    let app = Router::new()
        .route(
            &format!("/v1/{}/{{content_id}}", content_type),
            get({
                let valid = valid_content.clone();
                let ct = content_type.to_string();
                move |Path(content_id): Path<Uuid>| {
                    let valid = valid.clone();
                    let ct = ct.clone();
                    async move {
                        if valid
                            .get(&ct)
                            .is_some_and(|ids| ids.contains(&content_id))
                        {
                            Ok(Json(ContentResponse {
                                id: content_id,
                                title: format!("Sample {} {}", ct, content_id),
                                content_type: ct,
                            }))
                        } else {
                            Err(StatusCode::NOT_FOUND)
                        }
                    }
                }
            }),
        )
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status": "ok"})) }),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("  {} API on port {}", content_type, port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Serialize)]
struct AuthResponse {
    user_id: Uuid,
    username: String,
}

async fn serve_profile_api(
    port: u16,
    valid_tokens: Arc<HashMap<String, Uuid>>,
) {
    let app = Router::new()
        .route(
            "/v1/auth/validate",
            get({
                let tokens = valid_tokens.clone();
                move |headers: HeaderMap| {
                    let tokens = tokens.clone();
                    async move {
                        let token = headers
                            .get("Authorization")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.strip_prefix("Bearer "));

                        match token {
                            Some(t) => {
                                if let Some(user_id) = tokens.get(t) {
                                    Ok(Json(AuthResponse {
                                        user_id: *user_id,
                                        username: format!("user_{}", &user_id.to_string()[..8]),
                                    }))
                                } else {
                                    Err(StatusCode::UNAUTHORIZED)
                                }
                            }
                            None => Err(StatusCode::UNAUTHORIZED),
                        }
                    }
                }
            }),
        )
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status": "ok"})) }),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("  Profile API on port {}", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
