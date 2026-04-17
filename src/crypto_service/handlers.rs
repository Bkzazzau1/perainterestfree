use crate::auth::{jwt::Claims, security::verify_value};
use crate::crypto_provider_client::BrailsSendPayload;
use crate::crypto_service::models::{CryptoSendPayload, QuoteResponse}; // <-- Merged imports
use crate::{error::AppError, AppState};
use axum::{
    extract::{ConnectInfo, Path, Query, State}, // <-- Merged imports
    http::StatusCode,
    response::IntoResponse,
    Extension,
    Json,
};
use axum_extra::{headers::UserAgent, TypedHeader};
use serde::Deserialize; // <-- Added for QuoteQuery
use serde_json::json;
use std::net::SocketAddr;
use tracing::{debug, info};
use uuid::Uuid; // <-- ADD THIS

/// Handler for POST /api/v1/crypto/deposit-address/:asset/:chain (Source 24)
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn get_receive_address(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((asset, chain)): Path<(String, String)>, // <-- Get from path
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;

    // 1. Check if we already have an address
    let existing = sqlx::query!(
        "SELECT address, memo_tag FROM crypto_addresses WHERE user_id = $1 AND asset = $2 AND network = $3",
        user_id, asset, chain
    )
    .fetch_optional(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;

    if let Some(record) = existing {
        // --- ADDED ---
        debug!(user_id = %user_id, %asset, %chain, "Found cached deposit address");
        // -------------
        return Ok((
            StatusCode::OK,
            Json(json!({ "address": record.address, "memoTag": record.memo_tag })),
        ));
    }

    // 2. Get user email (required by Brails, Source 126)
    let user = sqlx::query!("SELECT email FROM users WHERE id = $1", user_id)
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

    // 4. Get one from the provider
    let new_addr = state
        .crypto_provider_client
        .get_deposit_address(api_key, &asset, &chain, &user.email)
        .await
        .map_err(AppError::ProviderError)?;

    // 5. Save it
    sqlx::query!(
        "INSERT INTO crypto_addresses (user_id, asset, network, address) VALUES ($1, $2, $3, $4)",
        user_id,
        asset,
        chain,
        new_addr.address
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- ADDED ---
    info!(user_id = %user_id, %asset, %chain, "Generated new deposit address");
    // -------------

    Ok((
        StatusCode::OK,
        Json(json!({ "address": new_addr.address, "memoTag": null })),
    ))
}

// --- Logic from old file, kept for convert_service router ---
#[derive(Deserialize)]
pub struct QuoteQuery {
    from: String,
    to: String,
}

/// Handler for GET /api/v1/crypto/quote
/// This is now also used by '/api/v1/convert/quote'
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn get_quote(
    State(state): State<AppState>,
    Query(query): Query<QuoteQuery>,
) -> Result<impl IntoResponse, AppError> {
    // --- ADDED ---
    debug!(from = %query.from, to = %query.to, "Fetching quote");
    // -------------

    // We pivot all quotes through USD
    let from_rate = if query.from == "USD" {
        1.0
    } else {
        1.0 / state
            .crypto_provider_client
            .get_quote("USD", &query.from)
            .await
            .map_err(AppError::ProviderError)?
    };

    let to_rate = if query.to == "USD" {
        1.0
    } else {
        state
            .crypto_provider_client
            .get_quote("USD", &query.to)
            .await
            .map_err(AppError::ProviderError)?
    };

    let rate = from_rate * to_rate;

    let response = QuoteResponse {
        from_asset: query.from,
        to_asset: query.to,
        rate,
    };

    Ok((StatusCode::OK, Json(response)))
}
// --- End of logic from old file ---

/// Handler for POST /api/v1/crypto/send (Source 23)
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn send_crypto(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(ip): ConnectInfo<SocketAddr>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    Json(payload): Json<CryptoSendPayload>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;

    // 1. Verify PIN
    let pin_hash = sqlx::query!("SELECT pin_hash FROM users WHERE id = $1", user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?
        .and_then(|u| u.pin_hash)
        .ok_or(AppError::ProviderError("PIN_NOT_SET".to_string()))?;

    if !verify_value(payload.pin, pin_hash).await? {
        // --- ADDED ---
        debug!(user_id = %user_id, "Crypto send failed: incorrect PIN");
        // -------------
        return Err(AppError::InvalidCredentials);
    }

    // 2. Convert Pera amount (major) to Brails amount (minor) (Source 73)
    let amount_minor = (payload.amount * 100.0).round() as i64;
    if amount_minor <= 0 {
        return Err(AppError::ProviderError("Invalid amount".to_string()));
    }

    let ip_address = ip.to_string();
    let user_agent_str = user_agent.to_string();

    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    // --- 3. (NEW) Fraud Check ---
    let (counterparty, asset_key) = (payload.to_address.clone(), payload.asset.clone());
    let assessment = crate::fraud_service::service::check_payment_risk(
        &mut tx,
        user_id,
        amount_minor,
        "crypto",
        &counterparty,
        "CRYPTO", // Use "CRYPTO" as country
        &ip_address,
        &user_agent_str,
    )
    .await?;

    if assessment.decision == "BLOCK" {
        // --- ADDED ---
        info!(user_id = %user_id, score = assessment.risk_score, "Crypto send blocked by fraud check");
        // -------------
        // TODO: Log alert
        tx.commit().await.ok(); // Commit to save the fraud check log
        return Err(AppError::TransactionDeclined(
            "Transaction blocked by risk policy".to_string(),
        ));
    }
    // -------------------------

    // 4. Get USD Wallet and Lock Row
    let wallet = sqlx::query!(
        "SELECT id, balance_minor FROM wallets WHERE user_id = $1 AND currency = 'USD' FOR UPDATE",
        user_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError("USD wallet not found".to_string()))?;

    // 5. Check Balance
    if wallet.balance_minor < amount_minor {
        tx.rollback().await.ok();
        // --- ADDED ---
        debug!(user_id = %user_id, "Crypto send failed: insufficient USD funds");
        // -------------
        return Err(AppError::ProviderError(
            "Insufficient USD funds".to_string(),
        ));
    }

    // 6. Debit Wallet
    sqlx::query!(
        "UPDATE wallets SET balance_minor = balance_minor - $1 WHERE id = $2",
        amount_minor,
        wallet.id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // 7. Get user email and API key
    let user_email = sqlx::query!("SELECT email FROM users WHERE id = $1", user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?
        .email;

    let settings = crate::admin_settings_service::service::get_all_settings(&state.db_pool).await?;
    let api_key = settings
        .get("brails_api_key")
        .ok_or(AppError::ProviderError(
            "Brails API key not set".to_string(),
        ))?;

    // 8. Log Transaction *before* API call, with a unique ID
    let new_tx = sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title,
            counterparty, reference, metadata, ip_address, user_agent
        )
        VALUES ($1, $2, 'crypto_send', $3, $4, 'USD', $5, $6, $7, $8, $9, $10)
        RETURNING id
        "#,
        user_id,
        wallet.id,
        assessment.status,
        -amount_minor,
        format!("Send {}", asset_key), // title
        counterparty,                  // counterparty
        Uuid::new_v4().to_string(),    // reference (this is our *internal* reference)
        json!({ "asset": asset_key, "network": payload.network, "score": assessment.risk_score }),
        ip_address,
        user_agent_str
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // 9. Build Brails Payload (Source 73)
    let brails_payload = BrailsSendPayload {
        amount: amount_minor,
        address: payload.to_address,
        chain: payload.network,
        reference: new_tx.id.to_string(), // Use *our* TX ID as the idempotency key/reference
        description: format!("Pera tx {}", new_tx.id),
        customer_email: user_email,
        callback_url: Some("https://api.pera.com/api/v1/webhooks/brails/crypto-send".to_string()),
    };

    // 10. Call Provider
    let provider_tx_id = state
        .crypto_provider_client
        .send_stablecoin(api_key, &asset_key, brails_payload)
        .await
        .map_err(|e| AppError::ProviderError(format!("Crypto send failed: {}", e)))?;

    // 11. (Optional) Update our tx with the provider's ID
    sqlx::query!(
        "UPDATE transactions SET reference = $1 WHERE id = $2",
        provider_tx_id,
        new_tx.id
    )
    .execute(&mut *tx)
    .await
    .ok(); // Don't fail if this errors

    // 12. Commit
    tx.commit().await.map_err(AppError::DatabaseError)?;

    // --- ADDED ---
    info!(
        user_id = %user_id,
        tx_id = %new_tx.id,
        asset = %asset_key,
        amount_minor = amount_minor,
        status = %assessment.status,
        "Processed crypto send"
    );
    // -------------

    Ok((
        StatusCode::OK,
        Json(json!({ "txId": new_tx.id, "status": assessment.status })),
    ))
}
