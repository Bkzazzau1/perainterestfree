use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use crate::{error::AppError, AppState};

/// Middleware to authenticate webhooks from Brails
/// This is a basic "shared secret" auth. A better way is HMAC.
pub async fn brails_auth(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {

    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(token) = auth_header {
        // We expect "Bearer <SECRET>"
        if token.starts_with("Bearer ") {
            let provided_secret = &token[7..];
            if provided_secret == state.brails_webhook_secret {
                // Success
                return Ok(next.run(req).await);
            }
        }
    }
    
    // Auth failed
    Err(AppError::Unauthorized)
}