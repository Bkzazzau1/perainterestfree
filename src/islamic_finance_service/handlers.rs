use axum::{
    extract::{ConnectInfo, State}, // <-- Added ConnectInfo
    http::StatusCode,
    response::IntoResponse,
    Json, Extension,
};
use axum::headers::UserAgent; // <-- Added
use axum::TypedHeader; // <-- Added
use std::net::SocketAddr; // <-- Added
use crate::{error::AppError, AppState};
use crate::auth::{jwt::Claims, security::verify_value};
use crate::islamic_finance_service::models::{ZakatRates, PayZakatPayload};
use serde_json::json;
use tracing::info;

/// Handler for GET /api/v1/islamic/zakat-rates
/// Provides the data for the Zakat calculator
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
    
    Ok((StatusCode::OK, Json(rates)))
}

/// Handler for POST /api/v1/islamic/pay-zakat
/// Atomically pays Zakat from the user's NGN wallet
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
        .await.map_err(AppError::DatabaseError)?
        .and_then(|u| u.pin_hash)
        .ok_or(AppError::ProviderError("PIN_NOT_SET".to_string()))?;

    if !verify_value(payload.pin, user_pin_hash).await? {
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
    let mut tx = state.db_pool.begin().await.map_err(AppError::DatabaseError)?;
    
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
    .await.map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError("NGN wallet not found".to_string()))?;

    // --- 6. Check Balance ---
    if wallet.balance_minor < amount_minor {
        tx.rollback().await.ok();
        return Err(AppError::ProviderError("Insufficient NGN funds for Zakat".to_string()));
    }

    // --- 7. Debit Wallet ---
    sqlx::query!(
        "UPDATE wallets SET balance_minor = balance_minor - $1 WHERE id = $2",
        amount_minor,
        wallet.id
    )
    .execute(&mut *tx)
    .await.map_err(AppError::DatabaseError)?;

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
        "Zakat Payment", // title
        beneficiary_name.clone(), // counterparty
        payload.beneficiary_id.clone(), // reference
        json!({ "beneficiary_id": payload.beneficiary_id }),
        ip_address, // <-- Save IP
        user_agent_str // <-- Save User-Agent
    )
    .fetch_one(&mut *tx)
    .await.map_err(AppError::DatabaseError)?;
    
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
    .await.map_err(AppError::DatabaseError)?;

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