use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct LikeResponse {
    pub liked: bool,
    pub content_type: String,
    pub content_id: Uuid,
    pub total_count: i64,
}

#[derive(Debug, Serialize)]
pub struct UnlikeResponse {
    pub liked: bool,
    pub content_type: String,
    pub content_id: Uuid,
    pub total_count: i64,
}

#[derive(Debug, Serialize)]
pub struct CountResponse {
    pub content_type: String,
    pub content_id: Uuid,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub liked: bool,
    pub content_type: String,
    pub content_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct BatchCountsResponse {
    pub counts: Vec<CountResponse>,
}

#[derive(Debug, Serialize)]
pub struct BatchStatusesResponse {
    pub statuses: Vec<StatusResponse>,
}

#[derive(Debug, Serialize)]
pub struct UserLikesResponse {
    pub items: Vec<UserLikeItem>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
pub struct UserLikeItem {
    pub content_type: String,
    pub content_id: Uuid,
    pub liked_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TopLikesResponse {
    pub items: Vec<TopLikeItem>,
}

#[derive(Debug, Serialize)]
pub struct TopLikeItem {
    pub content_type: String,
    pub content_id: Uuid,
    pub count: i64,
    pub rank: u32,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
