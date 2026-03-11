mod common;

use serde_json::json;

#[tokio::test]
async fn no_auth_header_returns_401() {
    let (app, pool, schema) = common::setup_test_app().await;

    let (status, body) = common::request(&app, "POST", "/v1/likes",
        Some(json!({"content_type": "post", "content_id": "731b0395-4888-4822-b516-05b4b7bf2089"})),
        None,
    ).await;

    assert_eq!(status, 401);
    assert_eq!(body["error"]["code"], "UNAUTHORIZED");

    common::teardown(&pool, &schema).await;
}

#[tokio::test]
async fn invalid_token_returns_401() {
    let (app, pool, schema) = common::setup_test_app().await;

    let (status, _) = common::request(&app, "POST", "/v1/likes",
        Some(json!({"content_type": "post", "content_id": "731b0395-4888-4822-b516-05b4b7bf2089"})),
        Some("invalid_token"),
    ).await;

    assert_eq!(status, 401);

    common::teardown(&pool, &schema).await;
}
