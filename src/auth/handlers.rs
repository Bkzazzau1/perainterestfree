use crate::auth::jwt::{self, Claims};
use crate::auth::models::{
    LoginUser, OtpRequest, PasswordResetConfirmRequest, PasswordResetRequest, RegisterUser,
    TokenResponse, UserResponse, VerifyOtpRequest,
};
use crate::auth::security::{hash_value, verify_value};
use crate::{error::AppError, AppState};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use chrono::{Duration, Utc};
use serde_json::json;
use tracing::{debug, info}; // <-- ADDED

fn ensure_valid_email(email: &str) -> Result<(), AppError> {
    let email = email.trim();
    let Some((local, domain)) = email.split_once('@') else {
        return Err(AppError::ProviderError("VALID_EMAIL_REQUIRED".to_string()));
    };

    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return Err(AppError::ProviderError("VALID_EMAIL_REQUIRED".to_string()));
    }

    Ok(())
}

/// Handler for POST /api/v1/auth/register
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterUser>,
) -> Result<impl IntoResponse, AppError> {
    let email = crate::otp_service::normalize_target("EMAIL", &payload.email);
    let phone = crate::otp_service::normalize_target("PHONE", &payload.phone);

    if crate::otp_service::require_verified(&state.db_pool, None, "REGISTER", "EMAIL", &email)
        .await
        .is_err()
    {
        crate::otp_service::mark_verified(
            &state.db_pool,
            None,
            "REGISTER",
            "EMAIL",
            &email,
            &payload.otp,
        )
        .await?;
    }

    let hashed_password = hash_value(payload.password).await?;

    sqlx::query!(
        "INSERT INTO users (email, phone, password_hash, email_verified_at) VALUES ($1, $2, $3, NOW())",
        email.clone(),
        phone,
        hashed_password
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.is_unique_violation() {
                // Log at a 'debug' level, as it's a user error
                debug!(email = %email, "User already exists");
                return AppError::InvalidCredentials; // Use a generic error
            }
        }
        AppError::DatabaseError(e)
    })?;

    crate::otp_service::consume(&state.db_pool, None, "REGISTER", "EMAIL", &email).await?;

    // Professional log with context
    info!(user_email = %email, "Created new user");

    Ok((StatusCode::CREATED, "User created successfully"))
}

/// Handler for POST /api/v1/auth/login
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginUser>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Fetch User and Verify Password
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users WHERE email = $1 OR phone = $1",
        payload.id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::InvalidCredentials)?;

    if !verify_value(payload.password, user.password_hash).await? {
        // --- ADDED ---
        debug!(login_id = %payload.id, "Login failed: incorrect password");
        // -------------
        return Err(AppError::InvalidCredentials);
    }

    // --- 2. Create a new Session ---
    let user_agent = headers
        .get("user-agent")
        .map_or("", |h| h.to_str().unwrap_or(""));

    let expires_at = Utc::now() + Duration::days(7);

    let session = sqlx::query!(
        r#"
        INSERT INTO user_sessions (user_id, user_agent, expires_at)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        user.id,
        user_agent,
        expires_at
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;
    // ---------------------------------

    // --- 3. Create JWT Token using the Session ID ---
    let token = jwt::create_token(user.id, session.id, &state.jwt_secret)?;

    // --- ADDED ---
    info!(user_id = %user.id, session_id = %session.id, "User logged in, session created");
    // -------------

    let response = TokenResponse { token };
    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /api/v1/auth/logout
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn logout(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>, // Get claims from middleware
) -> Result<impl IntoResponse, AppError> {
    // The 'jti' claim IS our session ID.
    let session_id = claims.jti;

    // Delete the session from the database.
    // The auth_middleware will now reject any future tokens with this 'jti'.
    sqlx::query!(
        "DELETE FROM user_sessions WHERE id = $1 AND user_id = $2",
        session_id,
        claims.sub
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- ADDED ---
    info!(user_id = %claims.sub, session_id = %session_id, "User logged out, session deleted");
    // -------------

    Ok((StatusCode::OK, "Logged out successfully"))
}

/// Handler for GET /api/v1/me
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn get_me(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>, // <-- Get claims from middleware
) -> Result<impl IntoResponse, AppError> {
    // The user ID 'sub' (subject) comes from the validated token
    let user_id = claims.sub;

    // Fetch user details from the database
    let user = sqlx::query_as!(
        UserResponse,
        r#"
        SELECT
            id,
            display_name,
            email,
            phone,
            kyc_status,
            email_verified_at IS NOT NULL AS "email_verified!",
            phone_verified_at IS NOT NULL AS "phone_verified!"
        FROM users
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| AppError::Unauthorized)?; // User in token not in DB?

    // --- ADDED ---
    // Using debug here as 'get_me' is called frequently and 'info' might be too noisy
    debug!(user_id = %user_id, "Fetched user profile (get_me)");
    // -------------

    Ok((StatusCode::OK, Json(user)))
}

/// Handler for POST /api/v1/auth/send-otp
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn send_otp(
    State(state): State<AppState>,
    Json(payload): Json<OtpRequest>,
) -> Result<impl IntoResponse, AppError> {
    let (channel, target) = if let Some(email) = payload.email {
        (
            "EMAIL",
            crate::otp_service::normalize_target("EMAIL", &email),
        )
    } else if let Some(phone) = payload.phone {
        (
            "PHONE",
            crate::otp_service::normalize_target("PHONE", &phone),
        )
    } else {
        return Err(AppError::ProviderError(
            "EMAIL_OR_PHONE_REQUIRED".to_string(),
        ));
    };

    if channel == "EMAIL" {
        ensure_valid_email(&target)?;
    }

    let code =
        crate::otp_service::create_otp(&state.db_pool, None, "REGISTER", channel, &target).await?;

    if channel == "EMAIL" {
        state
            .email_service
            .send_email(
                target.clone(),
                "Your Pera verification code".to_string(),
                format!(
                    "Your Pera verification code is {}. It expires in 10 minutes.",
                    code
                ),
            )
            .await;
    } else {
        debug!(target = %target, code = %code, "Generated phone OTP for registration");
    }

    Ok((StatusCode::OK, Json(json!({ "sent": true }))))
}

/// Handler for POST /api/v1/auth/password-reset/request
#[axum::debug_handler]
pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(payload): Json<PasswordResetRequest>,
) -> Result<impl IntoResponse, AppError> {
    let email = crate::otp_service::normalize_target("EMAIL", &payload.email);
    ensure_valid_email(&email)?;

    let user = sqlx::query!("SELECT id FROM users WHERE email = $1", &email)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?;

    if let Some(user) = user {
        let code = crate::otp_service::create_otp(
            &state.db_pool,
            Some(user.id),
            "RESET_PASSWORD",
            "EMAIL",
            &email,
        )
        .await?;

        state
            .email_service
            .send_email(
                email.clone(),
                "Reset your Pera password".to_string(),
                format!(
                    "Your Pera password reset code is {}. It expires in 10 minutes.",
                    code
                ),
            )
            .await;

        info!(user_id = %user.id, email = %email, "Queued password reset OTP email");
    } else {
        debug!(email = %email, "Password reset requested for unknown email");
    }

    Ok((StatusCode::OK, Json(json!({ "sent": true }))))
}

/// Handler for POST /api/v1/auth/verify-otp
#[axum::debug_handler]
pub async fn verify_otp(
    State(state): State<AppState>,
    Json(payload): Json<VerifyOtpRequest>,
) -> Result<impl IntoResponse, AppError> {
    let (channel, target) = if let Some(email) = payload.email {
        (
            "EMAIL",
            crate::otp_service::normalize_target("EMAIL", &email),
        )
    } else if let Some(phone) = payload.phone {
        (
            "PHONE",
            crate::otp_service::normalize_target("PHONE", &phone),
        )
    } else {
        return Err(AppError::ProviderError(
            "EMAIL_OR_PHONE_REQUIRED".to_string(),
        ));
    };

    crate::otp_service::mark_verified(
        &state.db_pool,
        None,
        "REGISTER",
        channel,
        &target,
        &payload.code,
    )
    .await?;

    Ok((StatusCode::OK, Json(json!({ "verified": true }))))
}

/// Handler for POST /api/v1/auth/password-reset/confirm
#[axum::debug_handler]
pub async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(payload): Json<PasswordResetConfirmRequest>,
) -> Result<impl IntoResponse, AppError> {
    let email = crate::otp_service::normalize_target("EMAIL", &payload.email);
    ensure_valid_email(&email)?;

    if payload.new_password.trim().len() < 6 {
        return Err(AppError::ProviderError("PASSWORD_TOO_SHORT".to_string()));
    }

    let user = sqlx::query!("SELECT id FROM users WHERE email = $1", &email)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::InvalidCredentials)?;

    crate::otp_service::mark_verified(
        &state.db_pool,
        Some(user.id),
        "RESET_PASSWORD",
        "EMAIL",
        &email,
        &payload.code,
    )
    .await?;

    let hashed_password = hash_value(payload.new_password.trim().to_string()).await?;

    sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE id = $2",
        hashed_password,
        user.id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    sqlx::query!("DELETE FROM user_sessions WHERE user_id = $1", user.id)
        .execute(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?;

    crate::otp_service::consume(
        &state.db_pool,
        Some(user.id),
        "RESET_PASSWORD",
        "EMAIL",
        &email,
    )
    .await?;

    info!(user_id = %user.id, "Password reset completed");

    Ok((StatusCode::OK, Json(json!({ "reset": true }))))
}
