mod common;

use serde_json::json;

#[tokio::test]
async fn batch_counts_mixed_valid_invalid() {
    let (app, pool, schema) = common::setup_test_app().await;
    let valid_id = "731b0395-4888-4822-b516-05b4b7bf2089";
    let unknown_id = "00000000-0000-0000-0000-000000000000";

    common::request(&app, "POST", "/v1/likes",
        Some(json!({"content_type": "post", "content_id": valid_id})),
        Some("tok_user_1"),
    ).await;

    let (status, body) = common::request(&app, "POST", "/v1/likes/batch/counts",
        Some(json!({
            "items": [
                {"content_type": "post", "content_id": valid_id},
                {"content_type": "post", "content_id": unknown_id}
            ]
        })),
        None,
    ).await;

    assert_eq!(status, 200);
    assert_eq!(body["counts"].as_array().unwrap().len(), 2);

    common::teardown(&pool, &schema).await;
}

#[tokio::test]
async fn batch_exceeding_100_returns_400() {
    let (app, pool, schema) = common::setup_test_app().await;

    let items: Vec<serde_json::Value> = (0..101)
        .map(|_| json!({"content_type": "post", "content_id": uuid::Uuid::new_v4().to_string()}))
        .collect();

    let (status, body) = common::request(&app, "POST", "/v1/likes/batch/counts",
        Some(json!({"items": items})),
        None,
    ).await;

    assert_eq!(status, 400);
    assert_eq!(body["error"]["code"], "BATCH_TOO_LARGE");

    common::teardown(&pool, &schema).await;
}
