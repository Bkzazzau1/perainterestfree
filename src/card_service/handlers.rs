use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json, Extension,
};
use crate::{error::AppError, AppState};
use crate::auth::jwt::Claims;
use crate::card_service::models::{
    CardItem, CreateVirtualCardPayload, RequestPhysicalCardPayload, CardToggles,
};
use uuid::Uuid;

/// Helper to check if KYC is verified
async fn check_kyc(
    pool: &sqlx::PgPool,
    user_id: Uuid
) -> Result<(), AppError> {
    let kyc_status = sqlx::query!("SELECT kyc_status FROM users WHERE id = $1", user_id)
        .fetch_optional(pool)
        .await.map_err(AppError::DatabaseError)?
        .map_or("".to_string(), |r| r.kyc_status);

    if kyc_status != "verified" {
        Err(AppError::KycIncomplete)
    } else {
        Ok(())
    }
}

/// Handler for GET /api/v1/cards
pub async fn get_cards(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let cards = sqlx::query_as!(
        CardItem,
        r#"
        SELECT
            id, kind, network, currency, holder_name,
            masked_pan, balance_minor, activated,
            frozen, allow_foreign, product
        FROM cards
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        claims.sub
    )
    .fetch_all(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    
    Ok((StatusCode::OK, Json(cards)))
}

/// Handler for POST /api/v1/cards/virtual
pub async fn create_virtual_card(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateVirtualCardPayload>,
) -> Result<impl IntoResponse, AppError> {
    
    // 1. Check KYC (as per cards_view.dart)
    check_kyc(&state.db_pool, claims.sub).await?;
    
    // 2. (MOCK) Call Brails
    // let brails_card = state.brails_client.create_card(type: "virtual", ...).await?;
    let mock_card_id = Uuid::new_v4().to_string();
    let mock_pan = format!("**** **** **** 1234");
    
    // 3. Save to DB
    let card = sqlx::query_as!(
        CardItem,
        r#"
        INSERT INTO cards (
            user_id, provider_card_id, kind, product, network,
            currency, holder_name, masked_pan, activated
        )
        VALUES ($1, $2, 'virtual', 'standard', $3, 'USD', 'Pera User', $4, true)
        RETURNING
            id, kind, network, currency, holder_name,
            masked_pan, balance_minor, activated,
            frozen, allow_foreign, product
        "#,
        claims.sub,
        mock_card_id,
        payload.network,
        mock_pan
    )
    .fetch_one(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;

    Ok((StatusCode::CREATED, Json(card)))
}

/// Handler for POST /api/v1/cards/physical
pub async fn request_physical_card(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<RequestPhysicalCardPayload>,
) -> Result<impl IntoResponse, AppError> {

    // 1. Check KYC
    check_kyc(&state.db_pool, claims.sub).await?;

    // 2. (MOCK) Call Brails
    let mock_card_id = Uuid::new_v4().to_string();
    let mock_pan = format!("**** **** **** 5678");

    // 3. Save Card and Request in a Transaction
    let mut tx = state.db_pool.begin().await.map_err(AppError::DatabaseError)?;

    let card = sqlx::query!(
        r#"
        INSERT INTO cards (
            user_id, provider_card_id, kind, product, network,
            currency, holder_name, masked_pan, activated
        )
        VALUES ($1, $2, 'physical', 'umrahPrepaid', $3, 'USD', $4, $5, false)
        RETURNING id
        "#,
        claims.sub,
        mock_card_id,
        payload.network,
        payload.full_name,
        mock_pan
    )
    .fetch_one(&mut *tx)
    .await.map_err(AppError::DatabaseError)?;
    
    sqlx::query!(
        r#"
        INSERT INTO physical_card_requests (
            user_id, card_id, delivery_type, full_name, phone,
            address, city, state_region
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        claims.sub,
        card.id,
        payload.delivery_type,
        payload.full_name,
        payload.phone,
        payload.address,
        payload.city,
        payload.state_region
    )
    .execute(&mut *tx)
    .await.map_err(AppError::DatabaseError)?;
    
    tx.commit().await.map_err(AppError::DatabaseError)?;

    Ok((StatusCode::CREATED, "Physical card request submitted"))
}

// --- Card Management Handlers ---

pub async fn freeze_card(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(card_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // (MOCK) await state.brails_client.freeze_card(card_id)
    sqlx::query!(
        "UPDATE cards SET frozen = true WHERE id = $1 AND user_id = $2",
        card_id,
        claims.sub
    )
    .execute(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    
    Ok(StatusCode::OK)
}

pub async fn unfreeze_card(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(card_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // (MOCK) await state.brails_client.unfreeze_card(card_id)
    sqlx::query!(
        "UPDATE cards SET frozen = false WHERE id = $1 AND user_id = $2",
        card_id,
        claims.sub
    )
    .execute(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    
    Ok(StatusCode::OK)
}

pub async fn set_card_toggles(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(card_id): Path<Uuid>,
    Json(payload): Json<CardToggles>,
) -> Result<impl IntoResponse, AppError> {
    // (MOCK) await state.brails_client.set_toggles(card_id, foreign: payload.allow_foreign)
    sqlx::query!(
        "UPDATE cards SET allow_foreign = $1 WHERE id = $2 AND user_id = $3",
        payload.allow_foreign,
        card_id,
        claims.sub
    )
    .execute(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    
    Ok(StatusCode::OK)
}