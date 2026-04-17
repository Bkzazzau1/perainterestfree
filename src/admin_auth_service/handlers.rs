use crate::admin_auth_service::models::{AdminClaims, AdminLoginPayload};
use crate::auth::models::TokenResponse; // Reuse the response struct
use crate::auth::security::verify_value; // Reuse our existing helper
use crate::{error::AppError, AppState};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use tracing::{debug, info}; // <-- ADD THIS

/// Handler for POST /api/v1/admin/login (FIXED)
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn admin_login(
    State(state): State<AppState>,
    Json(payload): Json<AdminLoginPayload>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Find the admin user and JOIN to get their role name
    let admin = sqlx::query!(
        r#"
        SELECT
            au.id,
            au.password_hash,
            ar.role_name
        FROM admin_users au
        JOIN admin_roles ar ON au.role_id = ar.id
        WHERE au.email = $1 AND au.is_active = true
        "#,
        payload.email
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::InvalidCredentials)?;

    // 2. Verify password
    if !verify_value(payload.password, admin.password_hash).await? {
        // --- ADDED ---
        debug!(email = %payload.email, "Admin login failed: incorrect password");
        // -------------
        return Err(AppError::InvalidCredentials);
    }

    // 3. Create Admin JWT
    let now = Utc::now();
    let claims = AdminClaims {
        sub: admin.id,
        role: admin.role_name, // <-- Use the role_name from the JOIN
        iat: now.timestamp(),
        exp: (now + Duration::hours(8)).timestamp(), // Shorter admin sessions
    };
    let key = EncodingKey::from_secret(state.jwt_secret.as_bytes());
    let token =
        encode(&Header::default(), &claims, &key).map_err(|_| AppError::TokenCreationError)?;

    // 4. Update last_login_at
    sqlx::query!(
        "UPDATE admin_users SET last_login_at = NOW() WHERE id = $1",
        admin.id
    )
    .execute(&state.db_pool)
    .await
    .ok(); // Don't fail the login if this fails

    // --- ADDED ---
    info!(admin_id = %admin.id, email = %payload.email, role = %claims.role, "Admin user logged in successfully");
    // -------------

    Ok((StatusCode::OK, Json(TokenResponse { token })))
}

/// Handler for GET /api/v1/admin/stats (A test endpoint)
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn get_admin_stats(
    Extension(claims): Extension<AdminClaims>,
) -> Result<impl IntoResponse, AppError> {
    // --- ADDED ---
    info!(admin_id = %claims.sub, role = %claims.role, "Viewed admin stats");
    // -------------

    // This handler will only run if admin_auth_middleware passes
    let response = format!("Welcome, admin {} (Role: {})", claims.sub, claims.role);
    Ok((StatusCode::OK, response))
}
