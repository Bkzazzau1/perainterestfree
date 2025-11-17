use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json, Extension,
};
use crate::{error::AppError, AppState};
use crate::auth::jwt::Claims;
use crate::sessions_service::models::UserSession;
use uuid::Uuid;

/// Handler for GET /api/v1/sessions
pub async fn get_sessions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    
    let current_session_id = claims.jti; // Get ID of the *current* token

    let sessions = sqlx::query!(
        r#"
        SELECT id, user_agent, ip_address, created_at, expires_at
        FROM user_sessions
        WHERE user_id = $1 AND status = 'active'
        ORDER BY created_at DESC
        "#,
        claims.sub
    )
    .fetch_all(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?
    .into_iter()
    .map(|row| UserSession {
        id: row.id,
        user_agent: row.user_agent.unwrap_or_default(),
        ip_address: row.ip_address.unwrap_or_default(),
        created_at: row.created_at,
        expires_at: row.expires_at,
        // Check if this session is the one making the request
        is_current_session: row.id == current_session_id,
    })
    .collect::<Vec<_>>();
    
    Ok((StatusCode::OK, Json(sessions)))
}

/// Handler for DELETE /api/v1/sessions/:id
pub async fn revoke_session(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {

    // This just deletes the session. The auth_middleware will handle the rest.
    sqlx::query!(
        "DELETE FROM user_sessions WHERE id = $1 AND user_id = $2",
        id,
        claims.sub // Ensure user can only delete their own sessions
    )
    .execute(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;

    Ok(StatusCode::NO_CONTENT)
}