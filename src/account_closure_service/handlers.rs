use crate::account_closure_service::models::ClosurePayload;
use crate::auth::{jwt::Claims, security::verify_value};
use crate::{error::AppError, AppState};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use tracing::info;

/// Handler for POST /api/v1/user/close-account
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn request_account_closure(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<ClosurePayload>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;

    // 1. Verify PIN
    let pin_hash = sqlx::query!("SELECT pin_hash FROM users WHERE id = $1", user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?
        .and_then(|u| u.pin_hash)
        .ok_or(AppError::ProviderError("PIN_NOT_SET".to_string()))?;

    if !verify_value(payload.pin, pin_hash).await? {
        return Err(AppError::InvalidCredentials);
    }

    // 2. Log the request
    sqlx::query!(
        "INSERT INTO account_closure_requests (user_id, reason) VALUES ($1, $2)",
        user_id,
        payload.reason
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // 3. (Optional) Set user's main status to 'closure_pending'
    sqlx::query!(
        "UPDATE users SET kyc_status = 'pending_closure' WHERE id = $1",
        user_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // 4. (TODO) Log out this user
    // We would also delete their current session
    sqlx::query!(
        "DELETE FROM user_sessions WHERE id = $1 AND user_id = $2",
        claims.jti,
        user_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- ADDED ---
    info!(user_id = %user_id, session_id = %claims.jti, "Account closure requested and session terminated");
    // -------------

    Ok((StatusCode::OK, "Account closure request submitted"))
}
