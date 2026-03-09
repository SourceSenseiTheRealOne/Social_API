use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::clients::ProfileClient;
use crate::errors::AppError;
use crate::models::like::UserInfo;

pub struct AuthState {
    pub profile_client: Arc<dyn ProfileClient>,
}

pub async fn auth_middleware(
    State(state): State<Arc<AuthState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    let user_info = state.profile_client.validate_token(token).await?;

    request.extensions_mut().insert(user_info);

    Ok(next.run(request).await)
}
