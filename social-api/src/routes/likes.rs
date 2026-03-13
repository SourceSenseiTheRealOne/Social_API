use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::like::UserInfo;
use crate::models::requests::*;
use crate::models::responses::*;
use crate::services::LikeService;

pub struct AppState {
    pub like_service: Arc<LikeService>,
}

pub async fn create_like(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserInfo>,
    Json(body): Json<CreateLikeRequest>,
) -> Result<Json<LikeResponse>, AppError> {
    let response = state
        .like_service
        .like(user.user_id, &body.content_type, body.content_id)
        .await?;
    Ok(Json(response))
}

pub async fn unlike(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserInfo>,
    Path((content_type, content_id)): Path<(String, Uuid)>,
) -> Result<Json<UnlikeResponse>, AppError> {
    let response = state
        .like_service
        .unlike(user.user_id, &content_type, content_id)
        .await?;
    Ok(Json(response))
}

pub async fn get_count(
    State(state): State<Arc<AppState>>,
    Path((content_type, content_id)): Path<(String, Uuid)>,
) -> Result<Json<CountResponse>, AppError> {
    let response = state
        .like_service
        .get_count(&content_type, content_id)
        .await?;
    Ok(Json(response))
}

pub async fn get_status(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserInfo>,
    Path((content_type, content_id)): Path<(String, Uuid)>,
) -> Result<Json<StatusResponse>, AppError> {
    let response = state
        .like_service
        .get_status(user.user_id, &content_type, content_id)
        .await?;
    Ok(Json(response))
}

pub async fn get_user_likes(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserInfo>,
    Query(query): Query<UserLikesQuery>,
) -> Result<Json<UserLikesResponse>, AppError> {
    let response = state
        .like_service
        .get_user_likes(user.user_id, query.cursor, query.limit, query.content_type)
        .await?;
    Ok(Json(response))
}

pub async fn batch_counts(
    State(state): State<Arc<AppState>>,
    Json(body): Json<BatchCountsRequest>,
) -> Result<Json<BatchCountsResponse>, AppError> {
    let response = state.like_service.get_batch_counts(body.items).await?;
    Ok(Json(response))
}

pub async fn batch_statuses(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserInfo>,
    Json(body): Json<BatchStatusesRequest>,
) -> Result<Json<BatchStatusesResponse>, AppError> {
    let response = state
        .like_service
        .get_batch_statuses(user.user_id, body.items)
        .await?;
    Ok(Json(response))
}

pub async fn get_top(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TopLikesQuery>,
) -> Result<Json<TopLikesResponse>, AppError> {
    let response = state
        .like_service
        .get_top_liked(query.content_type, query.window, query.limit)
        .await?;
    Ok(Json(response))
}
