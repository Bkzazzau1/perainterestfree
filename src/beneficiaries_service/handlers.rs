use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json, Extension,
};
use crate::{error::AppError, AppState};
use crate::auth::jwt::Claims;
use crate::beneficiaries_service::models::{Beneficiary, BeneficiaryPayload};
use uuid::Uuid;

/// Handler for GET /api/v1/beneficiaries
pub async fn get_beneficiaries(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    
    let beneficiaries = sqlx::query_as!(
        Beneficiary,
        "SELECT id, name, channel, provider, account FROM beneficiaries WHERE user_id = $1 ORDER BY name",
        claims.sub
    )
    .fetch_all(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;

    Ok((StatusCode::OK, Json(beneficiaries)))
}

/// Handler for POST /api/v1/beneficiaries
pub async fn add_beneficiary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<BeneficiaryPayload>,
) -> Result<impl IntoResponse, AppError> {

    let beneficiary = sqlx::query_as!(
        Beneficiary,
        r#"
        INSERT INTO beneficiaries (user_id, name, channel, provider, account)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, name, channel, provider, account
        "#,
        claims.sub,
        payload.name,
        payload.channel,
        payload.provider,
        payload.account
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.is_unique_violation() {
                return AppError::ProviderError("Beneficiary already exists".to_string());
            }
        }
        AppError::DatabaseError(e)
    })?;

    Ok((StatusCode::CREATED, Json(beneficiary)))
}

/// Handler for PUT /api/v1/beneficiaries/:id
pub async fn update_beneficiary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(payload): Json<BeneficiaryPayload>,
) -> Result<impl IntoResponse, AppError> {

    let beneficiary = sqlx::query_as!(
        Beneficiary,
        r#"
        UPDATE beneficiaries
        SET name = $1, channel = $2, provider = $3, account = $4
        WHERE id = $5 AND user_id = $6
        RETURNING id, name, channel, provider, account
        "#,
        payload.name,
        payload.channel,
        payload.provider,
        payload.account,
        id,
        claims.sub
    )
    .fetch_one(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?; // Fails if 'id' or 'user_id' doesn't match

    Ok((StatusCode::OK, Json(beneficiary)))
}

/// Handler for DELETE /api/v1/beneficiaries/:id
pub async fn delete_beneficiary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {

    let result = sqlx::query!(
        "DELETE FROM beneficiaries WHERE id = $1 AND user_id = $2",
        id,
        claims.sub
    )
    .execute(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    
    if result.rows_affected() == 0 {
        // This means the beneficiary didn't exist or didn't belong to the user
        return Err(AppError::ProviderError("Beneficiary not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT) // 204 No Content is standard for successful DELETE
}