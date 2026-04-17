use crate::{error::AppError, AppState};
use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};

/// Middleware to authenticate webhooks from Brails
/// This is a basic "shared secret" auth. A better way is HMAC.
#[allow(dead_code)]
pub async fn brails_auth(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
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

// --- ADD THIS ---
/// Middleware to authenticate webhooks from Payscribe
/// (This is a placeholder; Payscribe might use a different method)
#[allow(dead_code)]
pub async fn payscribe_auth(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // (MOCK) Check for a header or signature
    if let Some(secret) = req
        .headers()
        .get("x-payscribe-secret")
        .and_then(|v| v.to_str().ok())
    {
        if secret == state.payscribe_webhook_secret {
            return Ok(next.run(req).await);
        }
    }

    Err(AppError::Unauthorized)
}
// ----------------
