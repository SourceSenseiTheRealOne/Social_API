mod common;

use serde_json::json;

#[tokio::test]
async fn like_is_idempotent() {
    let (app, pool, schema) = common::setup_test_app().await;
    let content_id = "731b0395-4888-4822-b516-05b4b7bf2089";

    common::request(&app, "POST", "/v1/likes",
        Some(json!({"content_type": "post", "content_id": content_id})),
        Some("tok_user_1"),
    ).await;

    let (status, body) = common::request(&app, "POST", "/v1/likes",
        Some(json!({"content_type": "post", "content_id": content_id})),
        Some("tok_user_1"),
    ).await;

    assert_eq!(status, 200);
    assert_eq!(body["total_count"], 1);

    common::teardown(&pool, &schema).await;
}

#[tokio::test]
async fn unlike_nonexistent_is_idempotent() {
    let (app, pool, schema) = common::setup_test_app().await;
    let content_id = "731b0395-4888-4822-b516-05b4b7bf2089";

    let (status, body) = common::request(&app, "DELETE",
        &format!("/v1/likes/post/{}", content_id),
        None, Some("tok_user_1"),
    ).await;

    assert_eq!(status, 200);
    assert_eq!(body["liked"], false);

    common::teardown(&pool, &schema).await;
}
