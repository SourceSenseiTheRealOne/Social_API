use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateLikeRequest {
    pub content_type: String,
    pub content_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct BatchCountsRequest {
    pub items: Vec<ContentRef>,
}

#[derive(Debug, Deserialize)]
pub struct BatchStatusesRequest {
    pub items: Vec<ContentRef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContentRef {
    pub content_type: String,
    pub content_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UserLikesQuery {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub content_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TopLikesQuery {
    pub content_type: Option<String>,
    pub window: Option<super::like::TimeWindow>,
    pub limit: Option<u32>,
}
