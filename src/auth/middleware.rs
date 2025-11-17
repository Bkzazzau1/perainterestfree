use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use crate::{auth::jwt, error::AppError, AppState};

/// Axum middleware for authenticating requests.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {

    // 1. Extract the token
    let token = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_value| {
            if auth_value.starts_with("Bearer ") {
                Some(auth_value[7..].to_owned())
            } else {
                None
            }
        })
        .ok_or(AppError::Unauthorized)?;

    // 2. Decode and validate the token (checks expiry, signature)
    let claims = jwt::decode_token(&token, &state.jwt_secret)?;
    
    // --- 3. (NEW) Check if the session is valid in the database ---
    let session_status = sqlx::query!(
        "SELECT status FROM user_sessions WHERE id = $1 AND user_id = $2 AND expires_at > NOW()",
        claims.jti, // The session ID from the token
        claims.sub  // The user ID
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?
    .map_or("".to_string(), |r| r.status); // Get status, default to empty string if not found

    if session_status != "active" {
        // This session was logged out, revoked, or expired.
        // The token is now invalid.
        return Err(AppError::Unauthorized);
    }
    // -------------------------------------------------------------

    // 4. Insert claims for the handler
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}