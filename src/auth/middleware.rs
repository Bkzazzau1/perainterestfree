use axum::{
    body::Body,
    extract::State,
    http::{header, Request},
    middleware::Next,
    response::Response,
};

use crate::{auth::jwt, error::AppError, AppState};

/// Axum middleware for authenticating requests.
///
/// Works with axum 0.7 using `middleware::from_fn_with_state`.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // 1. Extract the token from Authorization: Bearer <token>
    let token = req
        .headers()
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

    // 2. Decode and validate the token (expiry, signature, etc.)
    let claims = jwt::decode_token(&token, &state.jwt_secret)?;

    // 3. Check if the session is still valid in the database
    let session_status = sqlx::query!(
        r#"
        SELECT status
        FROM user_sessions
        WHERE id = $1
          AND user_id = $2
          AND expires_at > NOW()
        "#,
        claims.jti, // session ID from token
        claims.sub  // user ID
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?
    .map_or_else(|| "".to_string(), |r| r.status);

    if session_status != "active" {
        // Session was logged out, revoked, or expired.
        return Err(AppError::Unauthorized);
    }

    // 4. Attach claims so handlers can access them
    req.extensions_mut().insert(claims);

    // 5. Continue the request chain
    Ok(next.run(req).await)
}
