use axum::{
    extract::{State, Request},
    http::StatusCode, 
    response::IntoResponse, 
    Json, Extension
};
use crate::auth::models::{LoginUser, RegisterUser, TokenResponse, UserResponse};
use crate::auth::jwt::{self, Claims};
use crate::{error::AppError, AppState};
use crate::auth::security::{hash_value, verify_value};
use tokio::task;
use uuid::Uuid;
use chrono::{Duration, Utc};
use tracing::{debug, info}; // <-- ADDED

/// Handler for POST /api/v1/auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterUser>,
) -> Result<impl IntoResponse, AppError> {
    
    let hashed_password = hash_value(payload.password).await?;
    
    sqlx::query!(
        "INSERT INTO users (email, phone, password_hash) VALUES ($1, $2, $3)",
        payload.email.to_lowercase(),
        payload.phone,
        hashed_password
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.is_unique_violation() {
                // Log at a 'debug' level, as it's a user error
                debug!(email = %payload.email, "User already exists");
                return AppError::InvalidCredentials; // Use a generic error
            }
        }
        AppError::DatabaseError(e)
    })?;

    // Professional log with context
    info!(user_email = %payload.email, "Created new user");
    
    Ok((StatusCode::CREATED, "User created successfully"))
}

/// Handler for POST /api/v1/auth/login
pub async fn login(
    State(state): State<AppState>,
    req: Request, // <-- Get the full request
    Json(payload): Json<LoginUser>,
) -> Result<impl IntoResponse, AppError> {

    // 1. Fetch User and Verify Password
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users WHERE email = $1 OR phone = $1",
        payload.id
    )
    .fetch_optional(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?
    .ok_or(AppError::InvalidCredentials)?;

    if !verify_value(payload.password, user.password_hash).await? {
        return Err(AppError::InvalidCredentials);
    }

    // --- 2. Create a new Session ---
    let user_agent = req.headers()
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
    .await.map_err(AppError::DatabaseError)?;
    // ---------------------------------

    // --- 3. Create JWT Token using the Session ID ---
    let token = jwt::create_token(user.id, session.id, &state.jwt_secret)?;

    let response = TokenResponse { token };
    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /api/v1/auth/logout
/// Revokes the current session
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
    .await.map_err(AppError::DatabaseError)?;

    Ok((StatusCode::OK, "Logged out successfully"))
}

/// Handler for GET /api/v1/me
/// Returns the authenticated user's details.
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
        SELECT id, email, phone, kyc_status
        FROM users
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| AppError::Unauthorized)?; // User in token not in DB?

    Ok((StatusCode::OK, Json(user)))
}

/// Handler for POST /api/v1/auth/send-otp
pub async fn send_otp(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // ...
    Ok((StatusCode::OK, "OTP sent"))
}