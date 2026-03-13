use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

use crate::circuit_breaker::CircuitBreaker;
use crate::errors::AppError;
use crate::models::like::UserInfo;

#[async_trait]
pub trait ProfileClient: Send + Sync {
    async fn validate_token(&self, token: &str) -> Result<UserInfo, AppError>;
}

pub struct HttpProfileClient {
    client: Client,
    base_url: String,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl HttpProfileClient {
    pub fn new(client: Client, base_url: String, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            client,
            base_url,
            circuit_breaker,
        }
    }
}

#[async_trait]
impl ProfileClient for HttpProfileClient {
    async fn validate_token(&self, token: &str) -> Result<UserInfo, AppError> {
        if self.circuit_breaker.check().is_err() {
            return Err(AppError::DependencyUnavailable {
                service: "profile-api".to_string(),
            });
        }

        let url = format!("{}/v1/auth/validate", self.base_url);
        let result = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;

        match result {
            Ok(response) => {
                if response.status().is_success() {
                    self.circuit_breaker.record_success();

                    #[derive(serde::Deserialize)]
                    struct AuthResponse {
                        user_id: uuid::Uuid,
                    }

                    let auth: AuthResponse = response
                        .json()
                        .await
                        .map_err(|_| AppError::Internal("Invalid auth response".into()))?;

                    Ok(UserInfo {
                        user_id: auth.user_id,
                    })
                } else if response.status() == reqwest::StatusCode::UNAUTHORIZED {
                    self.circuit_breaker.record_success();
                    Err(AppError::Unauthorized)
                } else {
                    self.circuit_breaker.record_failure();
                    Err(AppError::DependencyUnavailable {
                        service: "profile-api".to_string(),
                    })
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Profile API request failed");
                self.circuit_breaker.record_failure();
                Err(AppError::DependencyUnavailable {
                    service: "profile-api".to_string(),
                })
            }
        }
    }
}
