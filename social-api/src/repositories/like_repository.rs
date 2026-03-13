use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::cursor::Cursor;
use crate::models::like::{Like, LikeCount, TimeWindow};

#[async_trait]
pub trait LikeRepository: Send + Sync {
    async fn insert_like(
        &self,
        user_id: Uuid,
        content_type: &str,
        content_id: Uuid,
    ) -> Result<Option<Like>, AppError>;

    async fn delete_like(
        &self,
        user_id: Uuid,
        content_type: &str,
        content_id: Uuid,
    ) -> Result<bool, AppError>;

    async fn get_like_status(
        &self,
        user_id: Uuid,
        content_type: &str,
        content_id: Uuid,
    ) -> Result<bool, AppError>;

    async fn get_like_count(&self, content_type: &str, content_id: Uuid) -> Result<i64, AppError>;

    async fn get_user_likes(
        &self,
        user_id: Uuid,
        cursor: Option<Cursor>,
        limit: u32,
        content_type: Option<&str>,
    ) -> Result<Vec<Like>, AppError>;

    async fn get_batch_counts(&self, items: &[(String, Uuid)]) -> Result<Vec<LikeCount>, AppError>;

    async fn get_batch_statuses(
        &self,
        user_id: Uuid,
        items: &[(String, Uuid)],
    ) -> Result<Vec<(String, Uuid, bool)>, AppError>;

    async fn get_top_liked(
        &self,
        content_type: Option<&str>,
        window: TimeWindow,
        limit: u32,
    ) -> Result<Vec<LikeCount>, AppError>;

    async fn increment_count(&self, content_type: &str, content_id: Uuid) -> Result<i64, AppError>;

    async fn decrement_count(&self, content_type: &str, content_id: Uuid) -> Result<i64, AppError>;
}

pub struct PgLikeRepository {
    write_pool: PgPool,
    read_pool: PgPool,
}

impl PgLikeRepository {
    pub fn new(write_pool: PgPool, read_pool: PgPool) -> Self {
        Self {
            write_pool,
            read_pool,
        }
    }
}

#[async_trait]
impl LikeRepository for PgLikeRepository {
    async fn insert_like(
        &self,
        user_id: Uuid,
        content_type: &str,
        content_id: Uuid,
    ) -> Result<Option<Like>, AppError> {
        let like = sqlx::query_as::<_, Like>(
            r#"
            INSERT INTO likes (user_id, content_type, content_id)
            VALUES ($1, $2, $3)
            ON CONFLICT ON CONSTRAINT uq_user_content DO NOTHING
            RETURNING id, user_id, content_type, content_id, created_at
            "#,
        )
        .bind(user_id)
        .bind(content_type)
        .bind(content_id)
        .fetch_optional(&self.write_pool)
        .await?;

        Ok(like)
    }

    async fn delete_like(
        &self,
        user_id: Uuid,
        content_type: &str,
        content_id: Uuid,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM likes
            WHERE user_id = $1 AND content_type = $2 AND content_id = $3
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(content_type)
        .bind(content_id)
        .fetch_optional(&self.write_pool)
        .await?;

        Ok(result.is_some())
    }

    async fn get_like_status(
        &self,
        user_id: Uuid,
        content_type: &str,
        content_id: Uuid,
    ) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM likes
                WHERE user_id = $1 AND content_type = $2 AND content_id = $3
            )
            "#,
        )
        .bind(user_id)
        .bind(content_type)
        .bind(content_id)
        .fetch_one(&self.read_pool)
        .await?;

        Ok(exists)
    }

    async fn get_like_count(&self, content_type: &str, content_id: Uuid) -> Result<i64, AppError> {
        let count = sqlx::query_scalar::<_, Option<i64>>(
            r#"
            SELECT count FROM like_counts
            WHERE content_type = $1 AND content_id = $2
            "#,
        )
        .bind(content_type)
        .bind(content_id)
        .fetch_optional(&self.read_pool)
        .await?
        .flatten()
        .unwrap_or(0);

        Ok(count)
    }

    async fn get_user_likes(
        &self,
        user_id: Uuid,
        cursor: Option<Cursor>,
        limit: u32,
        content_type: Option<&str>,
    ) -> Result<Vec<Like>, AppError> {
        let likes = match (cursor, content_type) {
            (Some(c), Some(ct)) => {
                sqlx::query_as::<_, Like>(
                    r#"
                    SELECT id, user_id, content_type, content_id, created_at
                    FROM likes
                    WHERE user_id = $1
                      AND content_type = $2
                      AND (created_at, id) < ($3, $4)
                    ORDER BY created_at DESC, id DESC
                    LIMIT $5
                    "#,
                )
                .bind(user_id)
                .bind(ct)
                .bind(c.timestamp)
                .bind(c.id)
                .bind(limit as i64)
                .fetch_all(&self.read_pool)
                .await?
            }
            (Some(c), None) => {
                sqlx::query_as::<_, Like>(
                    r#"
                    SELECT id, user_id, content_type, content_id, created_at
                    FROM likes
                    WHERE user_id = $1
                      AND (created_at, id) < ($2, $3)
                    ORDER BY created_at DESC, id DESC
                    LIMIT $4
                    "#,
                )
                .bind(user_id)
                .bind(c.timestamp)
                .bind(c.id)
                .bind(limit as i64)
                .fetch_all(&self.read_pool)
                .await?
            }
            (None, Some(ct)) => {
                sqlx::query_as::<_, Like>(
                    r#"
                    SELECT id, user_id, content_type, content_id, created_at
                    FROM likes
                    WHERE user_id = $1 AND content_type = $2
                    ORDER BY created_at DESC, id DESC
                    LIMIT $3
                    "#,
                )
                .bind(user_id)
                .bind(ct)
                .bind(limit as i64)
                .fetch_all(&self.read_pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as::<_, Like>(
                    r#"
                    SELECT id, user_id, content_type, content_id, created_at
                    FROM likes
                    WHERE user_id = $1
                    ORDER BY created_at DESC, id DESC
                    LIMIT $2
                    "#,
                )
                .bind(user_id)
                .bind(limit as i64)
                .fetch_all(&self.read_pool)
                .await?
            }
        };

        Ok(likes)
    }

    async fn get_batch_counts(&self, items: &[(String, Uuid)]) -> Result<Vec<LikeCount>, AppError> {
        let content_types: Vec<&str> = items.iter().map(|(ct, _)| ct.as_str()).collect();
        let content_ids: Vec<Uuid> = items.iter().map(|(_, id)| *id).collect();

        let counts = sqlx::query_as::<_, LikeCount>(
            r#"
            SELECT lc.content_type, lc.content_id, COALESCE(lc.count, 0) as count
            FROM UNNEST($1::text[], $2::uuid[]) AS req(content_type, content_id)
            LEFT JOIN like_counts lc
              ON lc.content_type = req.content_type
              AND lc.content_id = req.content_id
            "#,
        )
        .bind(&content_types)
        .bind(&content_ids)
        .fetch_all(&self.read_pool)
        .await?;

        Ok(counts)
    }

    async fn get_batch_statuses(
        &self,
        user_id: Uuid,
        items: &[(String, Uuid)],
    ) -> Result<Vec<(String, Uuid, bool)>, AppError> {
        let content_types: Vec<&str> = items.iter().map(|(ct, _)| ct.as_str()).collect();
        let content_ids: Vec<Uuid> = items.iter().map(|(_, id)| *id).collect();

        let rows = sqlx::query_as::<_, (String, Uuid, bool)>(
            r#"
            SELECT req.content_type, req.content_id,
                   EXISTS(
                       SELECT 1 FROM likes l
                       WHERE l.user_id = $1
                         AND l.content_type = req.content_type
                         AND l.content_id = req.content_id
                   ) as liked
            FROM UNNEST($2::text[], $3::uuid[]) AS req(content_type, content_id)
            "#,
        )
        .bind(user_id)
        .bind(&content_types)
        .bind(&content_ids)
        .fetch_all(&self.read_pool)
        .await?;

        Ok(rows)
    }

    async fn get_top_liked(
        &self,
        content_type: Option<&str>,
        window: TimeWindow,
        limit: u32,
    ) -> Result<Vec<LikeCount>, AppError> {
        match window {
            TimeWindow::All => {
                let counts = if let Some(ct) = content_type {
                    sqlx::query_as::<_, LikeCount>(
                        r#"
                        SELECT content_type, content_id, count
                        FROM like_counts
                        WHERE content_type = $1 AND count > 0
                        ORDER BY count DESC
                        LIMIT $2
                        "#,
                    )
                    .bind(ct)
                    .bind(limit as i64)
                    .fetch_all(&self.read_pool)
                    .await?
                } else {
                    sqlx::query_as::<_, LikeCount>(
                        r#"
                        SELECT content_type, content_id, count
                        FROM like_counts
                        WHERE count > 0
                        ORDER BY count DESC
                        LIMIT $1
                        "#,
                    )
                    .bind(limit as i64)
                    .fetch_all(&self.read_pool)
                    .await?
                };
                Ok(counts)
            }
            _ => {
                let since = Utc::now() - window.into_duration().unwrap();
                let counts = if let Some(ct) = content_type {
                    sqlx::query_as::<_, LikeCount>(
                        r#"
                        SELECT content_type, content_id, COUNT(*) as count
                        FROM likes
                        WHERE content_type = $1 AND created_at >= $2
                        GROUP BY content_type, content_id
                        ORDER BY count DESC
                        LIMIT $3
                        "#,
                    )
                    .bind(ct)
                    .bind(since)
                    .bind(limit as i64)
                    .fetch_all(&self.read_pool)
                    .await?
                } else {
                    sqlx::query_as::<_, LikeCount>(
                        r#"
                        SELECT content_type, content_id, COUNT(*) as count
                        FROM likes
                        WHERE created_at >= $1
                        GROUP BY content_type, content_id
                        ORDER BY count DESC
                        LIMIT $2
                        "#,
                    )
                    .bind(since)
                    .bind(limit as i64)
                    .fetch_all(&self.read_pool)
                    .await?
                };
                Ok(counts)
            }
        }
    }

    async fn increment_count(&self, content_type: &str, content_id: Uuid) -> Result<i64, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO like_counts (content_type, content_id, count, updated_at)
            VALUES ($1, $2, 1, NOW())
            ON CONFLICT (content_type, content_id)
            DO UPDATE SET
                count = like_counts.count + 1,
                updated_at = NOW()
            RETURNING count
            "#,
        )
        .bind(content_type)
        .bind(content_id)
        .fetch_one(&self.write_pool)
        .await?;

        Ok(count)
    }

    async fn decrement_count(&self, content_type: &str, content_id: Uuid) -> Result<i64, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            UPDATE like_counts
            SET count = GREATEST(count - 1, 0),
                updated_at = NOW()
            WHERE content_type = $1 AND content_id = $2
            RETURNING count
            "#,
        )
        .bind(content_type)
        .bind(content_id)
        .fetch_optional(&self.write_pool)
        .await?
        .unwrap_or(0);

        Ok(count)
    }
}
