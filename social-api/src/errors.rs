use crate::models::responses::{ErrorBody, ErrorDetail};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Authorization required")]
    Unauthorized,

    #[error("Content not found: {content_type}/{content_id}")]
    ContentNotFound {
        content_type: String,
        content_id: String,
    },

    #[error("Unknown content type: {0}")]
    ContentTypeUnknown(String),

    #[error("Invalid content ID: {0}")]
    InvalidContentId(String),

    #[error("Batch too large: {size} items (max {max})")]
    BatchTooLarge { size: usize, max: usize },

    #[error("Invalid cursor")]
    InvalidCursor,

    #[error("Invalid time window: {0}")]
    InvalidWindow(String),

    #[error("Rate limit exceeded")]
    RateLimited { retry_after: u64 },

    #[error("Dependency unavailable: {service}")]
    DependencyUnavailable { service: String },

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            AppError::Unauthorized => "UNAUTHORIZED",
            AppError::ContentNotFound { .. } => "CONTENT_NOT_FOUND",
            AppError::ContentTypeUnknown(_) => "CONTENT_TYPE_UNKNOWN",
            AppError::InvalidContentId(_) => "INVALID_CONTENT_ID",
            AppError::BatchTooLarge { .. } => "BATCH_TOO_LARGE",
            AppError::InvalidCursor => "INVALID_CURSOR",
            AppError::InvalidWindow(_) => "INVALID_WINDOW",
            AppError::RateLimited { .. } => "RATE_LIMITED",
            AppError::DependencyUnavailable { .. } => "DEPENDENCY_UNAVAILABLE",
            AppError::Internal(_) => "INTERNAL_ERROR",
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::ContentNotFound { .. } => StatusCode::NOT_FOUND,
            AppError::ContentTypeUnknown(_) => StatusCode::BAD_REQUEST,
            AppError::InvalidContentId(_) => StatusCode::BAD_REQUEST,
            AppError::BatchTooLarge { .. } => StatusCode::BAD_REQUEST,
            AppError::InvalidCursor => StatusCode::BAD_REQUEST,
            AppError::InvalidWindow(_) => StatusCode::BAD_REQUEST,
            AppError::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            AppError::DependencyUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ErrorBody {
            error: ErrorDetail {
                code: self.code().to_string(),
                message: self.to_string(),
                request_id: None,
                details: None,
            },
        };

        let mut response = (status, Json(body)).into_response();
        if let AppError::RateLimited { retry_after } = &self {
            response.headers_mut().insert(
                "Retry-After",
                retry_after.to_string().parse().unwrap(),
            );
        }

        response
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!(error = %err, "Database error");
        AppError::Internal("Database error".to_string())
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        tracing::error!(error = %err, "Redis error");
        AppError::Internal("Cache error".to_string())
    }
}
