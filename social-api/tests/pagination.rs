mod common;

use serde_json::json;

#[tokio::test]
async fn cursor_pagination_traversal() {
    let (app, pool, schema) = common::setup_test_app().await;

    let content_ids: Vec<String> = vec![
        "a1b2c3d4-e5f6-4321-abcd-111111111111",
        "a1b2c3d4-e5f6-4321-abcd-222222222222",
        "a1b2c3d4-e5f6-4321-abcd-333333333333",
        "a1b2c3d4-e5f6-4321-abcd-444444444444",
        "a1b2c3d4-e5f6-4321-abcd-555555555555",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    for cid in &content_ids {
        common::request(
            &app,
            "POST",
            "/v1/likes",
            Some(json!({"content_type": "post", "content_id": cid})),
            Some("tok_user_1"),
        )
        .await;
    }

    // Page 1: limit=2
    let (status, body) = common::request(
        &app,
        "GET",
        "/v1/likes/user?limit=2",
        None,
        Some("tok_user_1"),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
    assert_eq!(body["has_more"], true);
    let cursor = body["next_cursor"].as_str().unwrap();

    // Page 2: use cursor
    let (status, body) = common::request(
        &app,
        "GET",
        &format!("/v1/likes/user?limit=2&cursor={}", cursor),
        None,
        Some("tok_user_1"),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
    assert_eq!(body["has_more"], true);

    // Page 3: last page
    let cursor = body["next_cursor"].as_str().unwrap();
    let (status, body) = common::request(
        &app,
        "GET",
        &format!("/v1/likes/user?limit=2&cursor={}", cursor),
        None,
        Some("tok_user_1"),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["has_more"], false);

    common::teardown(&pool, &schema).await;
}
