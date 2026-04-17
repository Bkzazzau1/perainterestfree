use crate::auth::jwt::Claims;
use crate::card_service::models::{
    CardItem, CardToggles, CreateVirtualCardPayload, RequestPhysicalCardPayload,
};
use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde_json::json; // <-- Added
use uuid::Uuid;
// --- ADD THESE ---
use crate::brails_client::BrailsRegisterUserPayload;
use tracing::{debug, info}; // <-- UPDATED
                            // -----------------

/// Helper to check if KYC is verified
async fn check_kyc(pool: &sqlx::PgPool, user_id: Uuid) -> Result<(), AppError> {
    let kyc_status = sqlx::query!("SELECT kyc_status FROM users WHERE id = $1", user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::DatabaseError)?
        .map_or("".to_string(), |r| r.kyc_status);

    if kyc_status != "verified" {
        // --- ADDED ---
        debug!(user_id = %user_id, "KYC check failed");
        // -------------
        Err(AppError::KycIncomplete)
    } else {
        Ok(())
    }
}

/// Handler for GET /api/v1/cards
#[axum::debug_handler] // <-- CORE FIX APPLIED
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
    .await
    .map_err(AppError::DatabaseError)?;

    // --- ADDED ---
    info!(user_id = %claims.sub, "Fetched user cards");
    // -------------

    Ok((StatusCode::OK, Json(cards)))
}

// --- ADD THIS NEW HANDLER ---
/// Handler for POST /api/v1/cards/users/register (Source 20)
/// This is the *real* KYC submission to Brails.
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn register_card_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;

    // 1. Get the user's saved profile data
    let profile = sqlx::query!("SELECT * FROM user_profiles WHERE user_id = $1", user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ProviderError(
            "KYC profile not found. Please submit first.".to_string(),
        ))?;

    // 2. Get user's auth info
    let user = sqlx::query!("SELECT email, phone FROM users WHERE id = $1", user_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?;

    // 3. Get Brails API key
    let settings = crate::admin_settings_service::service::get_all_settings(&state.db_pool).await?;
    let api_key = settings
        .get("brails_api_key")
        .ok_or(AppError::ProviderError(
            "Brails API key not set".to_string(),
        ))?;

    // 4. Decrypt sensitive data
    let bvn = profile
        .bvn_encrypted
        .and_then(|b| state.crypto_service.decrypt(&b).ok())
        .ok_or(AppError::ProviderError("BVN is required".to_string()))?;

    // (MOCK) We'd fetch the base64 photo from a file storage service
    let user_photo_base64 = "mock-base-64-string".to_string();

    // 5. Build the Brails payload (Source 335)
    let brails_payload = BrailsRegisterUserPayload {
        customer_email: user.email,
        id_number: bvn.clone(), // Using BVN as ID number
        id_type: "BVN".to_string(),
        first_name: profile
            .first_name
            .ok_or(AppError::ProviderError("First name required".to_string()))?,
        last_name: profile
            .surname
            .ok_or(AppError::ProviderError("Surname required".to_string()))?,
        phone_number: user.phone,
        city: "Lagos".to_string(),  // (Mock, get from profile)
        state: "Lagos".to_string(), // (Mock, get from profile)
        country: "NG".to_string(),
        bvn,
        user_photo: user_photo_base64,
    };

    // 6. Call Brails
    let brails_response = state
        .brails_client
        .register_card_user(api_key, brails_payload)
        .await
        .map_err(AppError::ProviderError)?;

    // 7. Save Brails ID and update our internal KYC status
    sqlx::query!(
        "UPDATE users SET brails_customer_id = $1, kyc_status = $2 WHERE id = $3",
        brails_response.user_id,
        brails_response.kyc_status.to_lowercase(),
        user_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    info!(user_id = %user_id, brails_id = %brails_response.user_id, "Submitted card user (KYC) to Brails");

    Ok((
        StatusCode::OK,
        Json(json!({ "kycStatus": brails_response.kyc_status })),
    ))
}
// --------------------------

/// Handler for POST /api/v1/cards/virtual (REFACTORED)
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn create_virtual_card(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateVirtualCardPayload>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Check *internal* KYC status (Source: cards_view.dart)
    check_kyc(&state.db_pool, claims.sub).await?;

    // 2. (MOCK) Call Brails POST /cards/ (Source 20)
    // let brails_card = state.brails_client.create_card(...).await?;
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
    .await
    .map_err(AppError::DatabaseError)?;

    // --- ADDED ---
    info!(user_id = %claims.sub, card_id = %card.id, network = %payload.network, "Created virtual card");
    // -------------

    Ok((StatusCode::CREATED, Json(card)))
}

/// Handler for POST /api/v1/cards/physical
#[axum::debug_handler] // <-- CORE FIX APPLIED
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
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

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
    .await
    .map_err(AppError::DatabaseError)?;

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
    .await
    .map_err(AppError::DatabaseError)?;

    tx.commit().await.map_err(AppError::DatabaseError)?;

    // --- ADDED ---
    info!(user_id = %claims.sub, card_id = %card.id, "Requested physical card");
    // -------------

    Ok((StatusCode::CREATED, "Physical card request submitted"))
}

// --- Card Management Handlers ---

#[axum::debug_handler] // <-- CORE FIX APPLIED
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
    .await
    .map_err(AppError::DatabaseError)?;

    // --- ADDED ---
    info!(user_id = %claims.sub, card_id = %card_id, "Froze card");
    // -------------

    Ok(StatusCode::OK)
}

#[axum::debug_handler] // <-- CORE FIX APPLIED
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
    .await
    .map_err(AppError::DatabaseError)?;

    // --- ADDED ---
    info!(user_id = %claims.sub, card_id = %card_id, "Unfroze card");
    // -------------

    Ok(StatusCode::OK)
}

#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn set_card_toggles(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(card_id): Path<Uuid>,
    Json(payload): Json<CardToggles>, // <-- Corrected typo from CardTLoggles
) -> Result<impl IntoResponse, AppError> {
    // (MOCK) await state.brails_client.set_toggles(card_id, foreign: payload.allow_foreign)
    sqlx::query!(
        "UPDATE cards SET allow_foreign = $1 WHERE id = $2 AND user_id = $3",
        payload.allow_foreign,
        card_id,
        claims.sub
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- ADDED ---
    info!(user_id = %claims.sub, card_id = %card_id, allow_foreign = %payload.allow_foreign, "Set card toggles");
    // -------------

    Ok(StatusCode::OK)
}
