use crate::auth::{jwt::Claims, security::verify_value};
use crate::islamic_finance_service::models::{
    approved_umrah_agencies, find_umrah_agency_by_code, find_umrah_agency_by_id, PayUmrahPayload,
    PayZakatPayload, ResolveAgencyQuery, ZakatRates,
};
use crate::{error::AppError, AppState};
use axum::{
    extract::{ConnectInfo, Query, State}, // <-- Added ConnectInfo
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Extension,
    Json,
};
use axum_extra::{headers::UserAgent, TypedHeader}; // <-- Added
use serde_json::json;
use std::net::SocketAddr; // <-- Added
use tracing::{debug, info}; // <-- UPDATED
use uuid::Uuid;

/// Handler for GET /api/v1/islamic/zakat-rates
/// Provides the data for the Zakat calculator
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn get_zakat_rates() -> Result<impl IntoResponse, AppError> {
    // --- TODO: Professional Implementation ---
    // In a real app, you would 'await' a service that fetches
    // live spot prices from a financial data provider.
    // -----------------------------------------

    // Mock data based on 'zakat_service.dart'
    let rates = ZakatRates {
        gold_per_gram: 95000.0,
        silver_per_gram: 1200.0,
    };

    // --- ADDED ---
    debug!("Fetched mock Zakat rates");
    // -------------

    Ok((StatusCode::OK, Json(rates)))
}

#[axum::debug_handler]
pub async fn get_umrah_agencies() -> Result<impl IntoResponse, AppError> {
    Ok((StatusCode::OK, Json(approved_umrah_agencies())))
}

#[axum::debug_handler]
pub async fn resolve_umrah_agency(
    Query(query): Query<ResolveAgencyQuery>,
) -> Result<impl IntoResponse, AppError> {
    let code = query.code.trim();
    if code.is_empty() {
        return Err(AppError::ProviderError("AGENCY_CODE_REQUIRED".to_string()));
    }

    let agency =
        find_umrah_agency_by_code(code).ok_or(AppError::NotFound("umrah_agency".to_string()))?;

    Ok((StatusCode::OK, Json(agency)))
}

#[axum::debug_handler]
pub async fn get_umrah_receipts(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let receipts = sqlx::query!(
        r#"
        SELECT
            up.id,
            up.agency_id,
            up.agency_name,
            up.amount_minor,
            up.paid_at,
            t.reference,
            t.status
        FROM umrah_payments up
        INNER JOIN transactions t ON t.id = up.transaction_id
        WHERE up.user_id = $1
        ORDER BY up.paid_at DESC
        LIMIT 50
        "#,
        claims.sub
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    let payload: Vec<_> = receipts
        .into_iter()
        .map(|receipt| {
            json!({
                "id": receipt.id,
                "agencyId": receipt.agency_id,
                "agencyName": receipt.agency_name,
                "amountMinor": receipt.amount_minor,
                "reference": receipt.reference,
                "status": receipt.status,
                "paidAt": receipt.paid_at,
            })
        })
        .collect();

    Ok((StatusCode::OK, Json(payload)))
}

/// Handler for POST /api/v1/islamic/pay-zakat
/// Atomically pays Zakat from the user's NGN wallet
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn pay_zakat(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(ip): ConnectInfo<SocketAddr>, // <-- Added IP
    TypedHeader(user_agent): TypedHeader<UserAgent>, // <-- Added User-Agent
    Json(payload): Json<PayZakatPayload>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;

    // --- 1. Security: Verify PIN ---
    let user_pin_hash = sqlx::query!("SELECT pin_hash FROM users WHERE id = $1", user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?
        .and_then(|u| u.pin_hash)
        .ok_or(AppError::ProviderError("PIN_NOT_SET".to_string()))?;

    if !verify_value(payload.pin, user_pin_hash).await? {
        // --- ADDED ---
        debug!(user_id = %user_id, "Zakat payment failed: incorrect PIN");
        // -------------
        return Err(AppError::InvalidCredentials);
    }

    // --- 2. Sanitize Amount ---
    let amount_minor = (payload.amount * 100.0).round() as i64;
    if amount_minor <= 0 {
        return Err(AppError::ProviderError("Invalid Zakat amount".to_string()));
    }

    // --- 3. (Mock) Find Zakat Beneficiary ---
    let beneficiary_name = "Approved Zakat Foundation".to_string();
    let ip_address = ip.to_string();
    let user_agent_str = user_agent.to_string();

    // --- 4. Start ATOMIC Transaction ---
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    // --- 5. Get NGN Wallet and Lock Row ---
    let wallet = sqlx::query!(
        r#"
        SELECT id, balance_minor
        FROM wallets
        WHERE user_id = $1 AND currency = 'NGN'
        FOR UPDATE
        "#,
        user_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError("NGN wallet not found".to_string()))?;

    // --- 6. Check Balance ---
    if wallet.balance_minor < amount_minor {
        tx.rollback().await.ok();
        // --- ADDED ---
        debug!(user_id = %user_id, "Zakat payment failed: insufficient NGN funds");
        // -------------
        return Err(AppError::ProviderError(
            "Insufficient NGN funds for Zakat".to_string(),
        ));
    }

    // --- 7. Debit Wallet ---
    sqlx::query!(
        "UPDATE wallets SET balance_minor = balance_minor - $1 WHERE id = $2",
        amount_minor,
        wallet.id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- 8. Create General Transaction Record (UPDATED) ---
    let new_tx = sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title,
            counterparty, reference, metadata, ip_address, user_agent
        )
        VALUES ($1, $2, 'zakat', 'completed', $3, 'NGN', $4, $5, $6, $7, $8, $9)
        RETURNING id
        "#,
        user_id,
        wallet.id,
        -amount_minor,
        "Zakat Payment",                // title
        beneficiary_name.clone(),       // counterparty
        payload.beneficiary_id.clone(), // reference
        json!({ "beneficiary_id": payload.beneficiary_id }),
        ip_address,     // <-- Save IP
        user_agent_str  // <-- Save User-Agent
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- 9. Create Specific Zakat Log ---
    sqlx::query!(
        r#"
        INSERT INTO zakat_donations (user_id, amount_minor, transaction_id, beneficiary)
        VALUES ($1, $2, $3, $4)
        "#,
        user_id,
        amount_minor,
        new_tx.id,
        beneficiary_name
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- 10. (MOCK) Pay out to Beneficiary ---

    // --- 11. Commit Transaction ---
    tx.commit().await.map_err(AppError::DatabaseError)?;

    // --- 12. Log ---
    info!(
        amount = amount_minor,
        user_id = %user_id,
        beneficiary_id = %payload.beneficiary_id,
        "Processed Zakat payment"
    );

    Ok((StatusCode::OK, "Zakat payment successful"))
}

#[axum::debug_handler]
pub async fn pay_umrah(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(ip): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<PayUmrahPayload>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;
    let agency = find_umrah_agency_by_id(&payload.agency_id)
        .ok_or(AppError::NotFound("umrah_agency".to_string()))?;

    let user_pin_hash = sqlx::query!("SELECT pin_hash FROM users WHERE id = $1", user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?
        .and_then(|u| u.pin_hash)
        .ok_or(AppError::ProviderError("PIN_NOT_SET".to_string()))?;

    if !verify_value(payload.pin, user_pin_hash).await? {
        debug!(user_id = %user_id, "Umrah payment failed: incorrect PIN");
        return Err(AppError::InvalidCredentials);
    }

    let amount_minor = (payload.amount * 100.0).round() as i64;
    if amount_minor <= 0 {
        return Err(AppError::ProviderError(
            "INVALID_UMRAH_PAYMENT_AMOUNT".to_string(),
        ));
    }

    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    let wallet = sqlx::query!(
        r#"
        SELECT id, balance_minor
        FROM wallets
        WHERE user_id = $1 AND currency = 'NGN'
        FOR UPDATE
        "#,
        user_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError("NGN_WALLET_NOT_FOUND".to_string()))?;

    if wallet.balance_minor < amount_minor {
        tx.rollback().await.ok();
        return Err(AppError::ProviderError(
            "INSUFFICIENT_NGN_FUNDS_FOR_UMRAH".to_string(),
        ));
    }

    sqlx::query!(
        "UPDATE wallets SET balance_minor = balance_minor - $1, updated_at = NOW() WHERE id = $2",
        amount_minor,
        wallet.id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    let reference = format!("UMR-{}", Uuid::new_v4().simple());
    let user_agent = headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let ip_address = ip.to_string();

    let transaction = sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title,
            counterparty, reference, metadata, ip_address, user_agent
        )
        VALUES ($1, $2, 'umrah_payment', 'completed', $3, 'NGN', $4, $5, $6, $7, $8, $9)
        RETURNING id, created_at
        "#,
        user_id,
        wallet.id,
        -amount_minor,
        format!("Umrah Payment • {}", agency.name),
        agency.name.clone(),
        reference.clone(),
        json!({
            "agencyId": agency.id,
            "agencyCode": agency.code,
            "service": "umrah_payment"
        }),
        ip_address,
        user_agent
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    sqlx::query!(
        r#"
        INSERT INTO umrah_payments (user_id, amount_minor, transaction_id, agency_name, agency_id)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        user_id,
        amount_minor,
        transaction.id,
        agency.name.clone(),
        agency.id.clone()
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    tx.commit().await.map_err(AppError::DatabaseError)?;

    info!(
        user_id = %user_id,
        agency_id = %agency.id,
        amount_minor = amount_minor,
        "Processed Umrah payment"
    );

    Ok((
        StatusCode::OK,
        Json(json!({
            "reference": reference,
            "agencyId": agency.id,
            "agencyName": agency.name,
            "amountMinor": amount_minor,
            "status": "completed",
            "paidAt": transaction.created_at,
        })),
    ))
}
