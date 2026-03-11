mod common;

use serde_json::json;

#[tokio::test]
async fn full_like_lifecycle() {
    let (app, pool, schema) = common::setup_test_app().await;
    let content_id = "731b0395-4888-4822-b516-05b4b7bf2089";

    // 1. Count starts at 0
    let (status, body) = common::request(
        &app, "GET",
        &format!("/v1/likes/post/{}/count", content_id),
        None, None,
    ).await;
    assert_eq!(status, 200);
    assert_eq!(body["count"], 0);

    // 2. Like it
    let (status, body) = common::request(
        &app, "POST", "/v1/likes",
        Some(json!({"content_type": "post", "content_id": content_id})),
        Some("tok_user_1"),
    ).await;
    assert_eq!(status, 200);
    assert_eq!(body["liked"], true);
    assert_eq!(body["total_count"], 1);

    // 3. Count is now 1
    let (status, body) = common::request(
        &app, "GET",
        &format!("/v1/likes/post/{}/count", content_id),
        None, None,
    ).await;
    assert_eq!(status, 200);
    assert_eq!(body["count"], 1);

    // 4. Status shows liked
    let (status, body) = common::request(
        &app, "GET",
        &format!("/v1/likes/post/{}/status", content_id),
        None, Some("tok_user_1"),
    ).await;
    assert_eq!(status, 200);
    assert_eq!(body["liked"], true);

    // 5. Unlike
    let (status, body) = common::request(
        &app, "DELETE",
        &format!("/v1/likes/post/{}", content_id),
        None, Some("tok_user_1"),
    ).await;
    assert_eq!(status, 200);
    assert_eq!(body["liked"], false);
    assert_eq!(body["total_count"], 0);

    // 6. Count back to 0
    let (status, body) = common::request(
        &app, "GET",
        &format!("/v1/likes/post/{}/count", content_id),
        None, None,
    ).await;
    assert_eq!(status, 200);
    assert_eq!(body["count"], 0);

    common::teardown(&pool, &schema).await;
}
