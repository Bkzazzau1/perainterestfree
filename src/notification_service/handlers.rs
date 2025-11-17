use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json, Extension,
};
use crate::{error::AppError, AppState};
use crate::auth::jwt::Claims;
use crate::notification_service::models::{Notification, UnreadCount};
use uuid::Uuid;

/// Handler for GET /api/v1/notifications
pub async fn get_notifications(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let notifications = sqlx::query_as!(
        Notification,
        "SELECT id, title, body, is_read, created_at FROM notifications WHERE user_id = $1 ORDER BY created_at DESC LIMIT 30",
        claims.sub
    )
    .fetch_all(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    
    Ok((StatusCode::OK, Json(notifications)))
}

/// Handler for GET /api/v1/notifications/unread-count
pub async fn get_unread_count(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let count = sqlx::query_as!(
        UnreadCount,
        "SELECT COUNT(*) as count FROM notifications WHERE user_id = $1 AND is_read = false",
        claims.sub
    )
    .fetch_one(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    
    Ok((StatusCode::OK, Json(count)))
}

/// Handler for POST /api/v1/notifications/:id/read
pub async fn mark_as_read(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    sqlx::query!(
        "UPDATE notifications SET is_read = true WHERE id = $1 AND user_id = $2",
        id,
        claims.sub
    )
    .execute(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    
    Ok(StatusCode::OK)
}