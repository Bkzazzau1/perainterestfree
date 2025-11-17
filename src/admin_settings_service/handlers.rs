use axum::{extract::State, http::StatusCode, response::IntoResponse, Json, Extension};
use crate::{error::AppError, AppState};
use crate::admin_auth_service::models::AdminClaims;
use crate::admin_settings_service::{
    models::UpdateSettingsPayload,
    service::get_all_settings,
};
use tracing::info;

/// Handler for GET /api/v1/admin/settings
pub async fn get_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<AdminClaims>,
) -> Result<impl IntoResponse, AppError> {
    // Check for super_admin role (Section 3 of your spec)
    if claims.role != "super_admin" {
        return Err(AppError::Unauthorized);
    }
    
    let settings = get_all_settings(&state.db_pool).await?;
    Ok((StatusCode::OK, Json(settings)))
}

/// Handler for POST /api/v1/admin/settings
pub async fn update_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<AdminClaims>,
    Json(payload): Json<UpdateSettingsPayload>,
) -> Result<impl IntoResponse, AppError> {
    if claims.role != "super_admin" {
        return Err(AppError::Unauthorized);
    }

    let mut tx = state.db_pool.begin().await.map_err(AppError::DatabaseError)?;
    
    for (key, value) in payload.settings {
        sqlx::query!(
            r#"
            INSERT INTO system_settings (key, value, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (key) DO UPDATE SET
                value = EXCLUDED.value,
                updated_at = NOW()
            "#,
            key,
            value
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;
    }
    
    tx.commit().await.map_err(AppError::DatabaseError)?;
    
    info!(admin_id = %claims.sub, "Updated system settings");
    Ok((StatusCode::OK, "Settings updated"))
}