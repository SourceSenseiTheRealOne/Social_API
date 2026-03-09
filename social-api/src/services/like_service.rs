use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::cache::LikeCache;
use crate::clients::ContentClient;
use crate::errors::AppError;
use crate::models::cursor::Cursor;
use crate::models::like::{LikeEvent, LikeEventType, TimeWindow};
use crate::models::requests::ContentRef;
use crate::models::responses::*;
use crate::repositories::LikeRepository;

const MAX_BATCH_SIZE: usize = 100;
const DEFAULT_PAGE_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 100;
const DEFAULT_LEADERBOARD_SIZE: u32 = 10;
const MAX_LEADERBOARD_SIZE: u32 = 100;

pub struct LikeService {
    repo: Arc<dyn LikeRepository>,
    cache: Arc<dyn LikeCache>,
    content_client: Arc<dyn ContentClient>,
    event_tx: broadcast::Sender<LikeEvent>,
}

impl LikeService {
    pub fn new(
        repo: Arc<dyn LikeRepository>,
        cache: Arc<dyn LikeCache>,
        content_client: Arc<dyn ContentClient>,
        event_tx: broadcast::Sender<LikeEvent>,
    ) -> Self {
        Self {
            repo,
            cache,
            content_client,
            event_tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LikeEvent> {
        self.event_tx.subscribe()
    }

    pub async fn like(
        &self,
        user_id: Uuid,
        content_type: &str,
        content_id: Uuid,
    ) -> Result<LikeResponse, AppError> {
        let valid = self
            .content_client
            .validate_content(content_type, content_id)
            .await?;
        if !valid {
            return Err(AppError::ContentNotFound {
                content_type: content_type.to_string(),
                content_id: content_id.to_string(),
            });
        }

        let new_like = self.repo.insert_like(user_id, content_type, content_id).await?;

        let total_count = if new_like.is_some() {
            let db_count = self.repo.increment_count(content_type, content_id).await?;
            self.cache.increment_count(content_type, content_id).await;

            let _ = self.event_tx.send(LikeEvent {
                event_type: LikeEventType::Liked,
                content_type: content_type.to_string(),
                content_id,
                user_id,
                total_count: db_count,
            });

            db_count
        } else {
            self.get_count_with_cache(content_type, content_id).await?
        };

        Ok(LikeResponse {
            liked: true,
            content_type: content_type.to_string(),
            content_id,
            total_count,
        })
    }

    pub async fn unlike(
        &self,
        user_id: Uuid,
        content_type: &str,
        content_id: Uuid,
    ) -> Result<UnlikeResponse, AppError> {
        let was_deleted = self.repo.delete_like(user_id, content_type, content_id).await?;

        let total_count = if was_deleted {
            let db_count = self.repo.decrement_count(content_type, content_id).await?;
            self.cache.decrement_count(content_type, content_id).await;

            let _ = self.event_tx.send(LikeEvent {
                event_type: LikeEventType::Unliked,
                content_type: content_type.to_string(),
                content_id,
                user_id,
                total_count: db_count,
            });

            db_count
        } else {
            self.get_count_with_cache(content_type, content_id).await?
        };

        Ok(UnlikeResponse {
            liked: false,
            content_type: content_type.to_string(),
            content_id,
            total_count,
        })
    }

    pub async fn get_count(
        &self,
        content_type: &str,
        content_id: Uuid,
    ) -> Result<CountResponse, AppError> {
        let count = self.get_count_with_cache(content_type, content_id).await?;
        Ok(CountResponse {
            content_type: content_type.to_string(),
            content_id,
            count,
        })
    }

    pub async fn get_status(
        &self,
        user_id: Uuid,
        content_type: &str,
        content_id: Uuid,
    ) -> Result<StatusResponse, AppError> {
        let liked = self
            .repo
            .get_like_status(user_id, content_type, content_id)
            .await?;

        Ok(StatusResponse {
            liked,
            content_type: content_type.to_string(),
            content_id,
        })
    }

    pub async fn get_user_likes(
        &self,
        user_id: Uuid,
        cursor: Option<String>,
        limit: Option<u32>,
        content_type: Option<String>,
    ) -> Result<UserLikesResponse, AppError> {
        let limit = limit
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .min(MAX_PAGE_SIZE);

        let decoded_cursor = match cursor {
            Some(ref c) => {
                let cursor = Cursor::decode(c).ok_or(AppError::InvalidCursor)?;
                Some(cursor)
            }
            None => None,
        };

        let likes = self
            .repo
            .get_user_likes(
                user_id,
                decoded_cursor,
                limit + 1,
                content_type.as_deref(),
            )
            .await?;

        let has_more = likes.len() > limit as usize;
        let items: Vec<UserLikeItem> = likes
            .into_iter()
            .take(limit as usize)
            .map(|l| UserLikeItem {
                content_type: l.content_type,
                content_id: l.content_id,
                liked_at: l.created_at,
            })
            .collect();

        let next_cursor = if has_more {
            items.last().map(|item| {
                Cursor::new(item.liked_at, item.content_id).encode()
            })
        } else {
            None
        };

        Ok(UserLikesResponse {
            items,
            next_cursor,
            has_more,
        })
    }

    pub async fn get_batch_counts(
        &self,
        items: Vec<ContentRef>,
    ) -> Result<BatchCountsResponse, AppError> {
        if items.len() > MAX_BATCH_SIZE {
            return Err(AppError::BatchTooLarge {
                size: items.len(),
                max: MAX_BATCH_SIZE,
            });
        }

        let mut results = Vec::with_capacity(items.len());
        let mut cache_misses = Vec::new();

        for item in &items {
            if let Some(count) = self
                .cache
                .get_count(&item.content_type, item.content_id)
                .await
            {
                results.push(CountResponse {
                    content_type: item.content_type.clone(),
                    content_id: item.content_id,
                    count,
                });
            } else {
                cache_misses.push((item.content_type.clone(), item.content_id));
            }
        }

        if !cache_misses.is_empty() {
            let db_counts = self.repo.get_batch_counts(&cache_misses).await?;
            for lc in db_counts {
                self.cache
                    .set_count(&lc.content_type, lc.content_id, lc.count)
                    .await;
                results.push(CountResponse {
                    content_type: lc.content_type,
                    content_id: lc.content_id,
                    count: lc.count,
                });
            }
        }

        Ok(BatchCountsResponse { counts: results })
    }

    pub async fn get_batch_statuses(
        &self,
        user_id: Uuid,
        items: Vec<ContentRef>,
    ) -> Result<BatchStatusesResponse, AppError> {
        if items.len() > MAX_BATCH_SIZE {
            return Err(AppError::BatchTooLarge {
                size: items.len(),
                max: MAX_BATCH_SIZE,
            });
        }

        let query_items: Vec<(String, Uuid)> = items
            .into_iter()
            .map(|i| (i.content_type, i.content_id))
            .collect();

        let statuses = self.repo.get_batch_statuses(user_id, &query_items).await?;

        let response_statuses = statuses
            .into_iter()
            .map(|(ct, id, liked)| StatusResponse {
                liked,
                content_type: ct,
                content_id: id,
            })
            .collect();

        Ok(BatchStatusesResponse {
            statuses: response_statuses,
        })
    }

    pub async fn get_top_liked(
        &self,
        content_type: Option<String>,
        window: Option<TimeWindow>,
        limit: Option<u32>,
    ) -> Result<TopLikesResponse, AppError> {
        let limit = limit
            .unwrap_or(DEFAULT_LEADERBOARD_SIZE)
            .min(MAX_LEADERBOARD_SIZE);
        let window = window.unwrap_or(TimeWindow::All);

        let counts = self
            .repo
            .get_top_liked(content_type.as_deref(), window, limit)
            .await?;

        let items = counts
            .into_iter()
            .enumerate()
            .map(|(i, lc)| TopLikeItem {
                content_type: lc.content_type,
                content_id: lc.content_id,
                count: lc.count,
                rank: (i + 1) as u32,
            })
            .collect();

        Ok(TopLikesResponse { items })
    }

    async fn get_count_with_cache(
        &self,
        content_type: &str,
        content_id: Uuid,
    ) -> Result<i64, AppError> {
        if let Some(count) = self.cache.get_count(content_type, content_id).await {
            return Ok(count);
        }

        let lock_key = format!("count:{content_type}:{content_id}");
        if self
            .cache
            .acquire_stampede_lock(&lock_key, Duration::from_secs(5))
            .await
        {
            let count = self.repo.get_like_count(content_type, content_id).await?;
            self.cache.set_count(content_type, content_id, count).await;
            Ok(count)
        } else {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if let Some(count) = self.cache.get_count(content_type, content_id).await {
                Ok(count)
            } else {
                self.repo.get_like_count(content_type, content_id).await
            }
        }
    }
}
