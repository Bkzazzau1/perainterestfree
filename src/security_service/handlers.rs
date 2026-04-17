use crate::auth::{
    jwt::Claims,
    security::{hash_value, verify_value},
};
use crate::security_service::models::{ChangePasswordPayload, SetPinPayload, VerifyPinPayload};
use crate::{error::AppError, AppState};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde_json::json;
use tracing::{debug, info}; // <-- ADD THIS

/// Handler for POST /api/v1/security/change-password
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn change_password(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<ChangePasswordPayload>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Fetch user's current password hash
    let user = sqlx::query!("SELECT password_hash FROM users WHERE id = $1", claims.sub)
        .fetch_one(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?;

    // 2. Verify their 'old_password'
    let valid = verify_value(payload.old_password, user.password_hash).await?;
    if !valid {
        // --- ADDED ---
        debug!(user_id = %claims.sub, "Password change failed: incorrect old password");
        // -------------
        // Use 'InvalidCredentials' to prevent password-guessing
        return Err(AppError::InvalidCredentials);
    }

    // 3. Hash the new password
    // TODO: Add password strength validation (e.g., min 8 chars)
    if payload.new_password.len() < 8 {
        return Err(AppError::ProviderError(
            "Password must be at least 8 characters".to_string(),
        ));
    }
    let new_hash = hash_value(payload.new_password).await?;

    // 4. Update the database
    sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE id = $2",
        new_hash,
        claims.sub
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // TODO: Invalidate all other sessions for this user

    // --- ADDED ---
    info!(user_id = %claims.sub, "User changed password successfully");
    // -------------

    Ok((StatusCode::OK, "Password updated successfully"))
}

/// Handler for POST /api/v1/security/set-pin
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn set_pin(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<SetPinPayload>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Fetch user's password hash (to authorize this action)
    let user = sqlx::query!("SELECT password_hash FROM users WHERE id = $1", claims.sub)
        .fetch_one(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?;

    // 2. Verify their main password
    let valid = verify_value(payload.password, user.password_hash).await?;
    if !valid {
        // --- ADDED ---
        debug!(user_id = %claims.sub, "Set PIN failed: incorrect password");
        // -------------
        return Err(AppError::InvalidCredentials);
    }

    // 3. Validate the new PIN
    if payload.new_pin.len() != 4 || !payload.new_pin.chars().all(char::is_numeric) {
        return Err(AppError::ProviderError("PIN must be 4 digits".to_string()));
    }

    // 4. Hash the new PIN
    let pin_hash = hash_value(payload.new_pin).await?;

    // 5. Update the 'pin_hash' column
    sqlx::query!(
        "UPDATE users SET pin_hash = $1 WHERE id = $2",
        pin_hash,
        claims.sub
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- ADDED ---
    info!(user_id = %claims.sub, "User set PIN successfully");
    // -------------

    Ok((StatusCode::OK, "PIN updated successfully"))
}

/// Handler for POST /api/v1/security/verify-pin
#[axum::debug_handler]
pub async fn verify_pin(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<VerifyPinPayload>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query!("SELECT pin_hash FROM users WHERE id = $1", claims.sub)
        .fetch_one(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?;

    let pin_hash = user
        .pin_hash
        .ok_or(AppError::ProviderError("PIN_NOT_SET".to_string()))?;

    let valid = verify_value(payload.pin, pin_hash).await?;
    if !valid {
        debug!(user_id = %claims.sub, "PIN verification failed");
        return Err(AppError::InvalidCredentials);
    }

    Ok((StatusCode::OK, Json(json!({ "valid": true }))))
}
