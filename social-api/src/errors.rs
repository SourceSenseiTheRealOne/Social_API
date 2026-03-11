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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn unauthorized_returns_401() {
        let error = AppError::Unauthorized;
        assert_eq!(error.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(error.code(), "UNAUTHORIZED");
    }

    #[test]
    fn content_not_found_returns_404() {
        let error = AppError::ContentNotFound {
            content_type: "post".into(),
            content_id: "abc".into(),
        };
        assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(error.code(), "CONTENT_NOT_FOUND");
    }

    #[test]
    fn batch_too_large_returns_400() {
        let error = AppError::BatchTooLarge { size: 200, max: 100 };
        assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(error.code(), "BATCH_TOO_LARGE");
    }

    #[test]
    fn rate_limited_returns_429() {
        let error = AppError::RateLimited { retry_after: 30 };
        assert_eq!(error.status_code(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(error.code(), "RATE_LIMITED");
    }

    #[test]
    fn dependency_unavailable_returns_503() {
        let error = AppError::DependencyUnavailable {
            service: "profile-api".into(),
        };
        assert_eq!(error.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error.code(), "DEPENDENCY_UNAVAILABLE");
    }

    #[tokio::test]
    async fn error_response_has_json_format() {
        let error = AppError::InvalidCursor;
        let response = error.into_response();
        let status = response.status();
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(json["error"]["code"].is_string());
        assert!(json["error"]["message"].is_string());
    }

    #[tokio::test]
    async fn rate_limited_has_retry_after_header() {
        let error = AppError::RateLimited { retry_after: 42 };
        let response = error.into_response();

        assert_eq!(
            response.headers().get("Retry-After").unwrap(),
            "42"
        );
    }
}
