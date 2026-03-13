use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;

use crate::cache::LikeCache;
use crate::circuit_breaker::CircuitBreaker;
use crate::errors::AppError;

#[async_trait]
pub trait ContentClient: Send + Sync {
    async fn validate_content(
        &self,
        content_type: &str,
        content_id: uuid::Uuid,
    ) -> Result<bool, AppError>;
}

pub struct HttpContentClient {
    client: Client,
    api_urls: HashMap<String, String>,
    circuit_breakers: HashMap<String, Arc<CircuitBreaker>>,
    cache: Arc<dyn LikeCache>,
}

impl HttpContentClient {
    pub fn new(
        client: Client,
        api_urls: HashMap<String, String>,
        circuit_breakers: HashMap<String, Arc<CircuitBreaker>>,
        cache: Arc<dyn LikeCache>,
    ) -> Self {
        Self {
            client,
            api_urls,
            circuit_breakers,
            cache,
        }
    }
}

#[async_trait]
impl ContentClient for HttpContentClient {
    async fn validate_content(
        &self,
        content_type: &str,
        content_id: uuid::Uuid,
    ) -> Result<bool, AppError> {
        let base_url = self
            .api_urls
            .get(content_type)
            .ok_or_else(|| AppError::ContentTypeUnknown(content_type.to_string()))?;

        if let Some(valid) = self
            .cache
            .get_content_validation(content_type, content_id)
            .await
        {
            return Ok(valid);
        }

        let cb = self
            .circuit_breakers
            .get(content_type)
            .ok_or_else(|| AppError::Internal("Missing circuit breaker".into()))?;

        if cb.check().is_err() {
            return Err(AppError::DependencyUnavailable {
                service: format!("content-api-{}", content_type),
            });
        }

        let url = format!("{}/v1/{}/{}", base_url, content_type, content_id);
        let result = self.client.get(&url).send().await;

        match result {
            Ok(response) => {
                if response.status().is_success() {
                    cb.record_success();
                    self.cache
                        .set_content_validation(content_type, content_id, true)
                        .await;
                    Ok(true)
                } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                    cb.record_success();
                    self.cache
                        .set_content_validation(content_type, content_id, false)
                        .await;
                    Ok(false)
                } else {
                    cb.record_failure();
                    Err(AppError::DependencyUnavailable {
                        service: format!("content-api-{}", content_type),
                    })
                }
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    content_type = %content_type,
                    "Content API request failed"
                );
                cb.record_failure();
                Err(AppError::DependencyUnavailable {
                    service: format!("content-api-{}", content_type),
                })
            }
        }
    }
}
