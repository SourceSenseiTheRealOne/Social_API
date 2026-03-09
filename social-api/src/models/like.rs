use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Like {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content_type: String,
    pub content_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LikeCount {
    pub content_type: String,
    pub content_id: Uuid,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LikeEvent {
    pub event_type: LikeEventType,
    pub content_type: String,
    pub content_id: Uuid,
    pub user_id: Uuid,
    pub total_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LikeEventType {
    Liked,
    Unliked,
}

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeWindow {
    #[serde(rename = "24h")]
    H24,
    #[serde(rename = "7d")]
    D7,
    #[serde(rename = "30d")]
    D30,
    All,
}

impl TimeWindow {
    pub fn to_duration(&self) -> Option<chrono::Duration> {
        match self {
            TimeWindow::H24 => Some(chrono::Duration::hours(24)),
            TimeWindow::D7 => Some(chrono::Duration::days(7)),
            TimeWindow::D30 => Some(chrono::Duration::days(30)),
            TimeWindow::All => None,
        }
    }
}
