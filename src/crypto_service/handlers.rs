use axum::{
    extract::{ConnectInfo, Query, State}, // <-- Added ConnectInfo
    http::StatusCode,
    response::IntoResponse,
    Json, Extension,
};
use axum::headers::UserAgent; // <-- Added
use axum::TypedHeader; // <-- Added
use std::net::SocketAddr; // <-- Added
use crate::{error::AppError, AppState};
use crate::auth::{jwt::Claims, security::verify_value};
use crate::crypto_service::models::{
    QuoteResponse, CryptoSendPayload
};
use serde::Deserialize;
use serde_json::json;
use tracing::info;

#[derive(Deserialize)]
pub struct AddressQuery {
    asset: String,
    network: String,
}

/// Handler for GET /api/v1/crypto/addresses
pub async fn get_receive_address(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<AddressQuery>,
) -> Result<impl IntoResponse, AppError> {
    
    // 1. Check if we already have an address
    let existing = sqlx::query!(
        "SELECT address, memo_tag FROM crypto_addresses WHERE user_id = $1 AND asset = $2 AND network = $3",
        claims.sub, query.asset, query.network
    )
    .fetch_optional(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    
    if let Some(record) = existing {
        return Ok((StatusCode::OK, Json(json!({ "address": record.address, "memoTag": record.memo_tag }))));
    }

    // 2. If not, get one from the provider
    let new_addr = state.crypto_provider_client
        .get_deposit_address(&query.asset, &query.network)
        .await
        .map_err(AppError::ProviderError)?;
        
    // 3. Save it
    sqlx::query!(
        "INSERT INTO crypto_addresses (user_id, asset, network, address, memo_tag) VALUES ($1, $2, $3, $4, $5)",
        claims.sub, query.asset, query.network, new_addr.address, new_addr.memo_tag
    )
    .execute(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;

    Ok((StatusCode::OK, Json(json!({ "address": new_addr.address, "memoTag": new_addr.memo_tag }))))
}

#[derive(Deserialize)]
pub struct QuoteQuery {
    from: String,
    to: String,
}

/// Handler for GET /api/v1/crypto/quote
/// This is now also used by '/api/v1/convert/quote'
pub async fn get_quote(
    State(state): State<AppState>,
    Query(query): Query<QuoteQuery>,
) -> Result<impl IntoResponse, AppError> {
    
    // We pivot all quotes through USD
    let from_rate = if query.from == "USD" {
        1.0
    } else {
        1.0 / state.crypto_provider_client.get_quote("USD", &query.from).await
            .map_err(AppError::ProviderError)?
    };
    
    let to_rate = if query.to == "USD" {
        1.0
    } else {
        state.crypto_provider_client.get_quote("USD", &query.to).await
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

/// Handler for POST /api/v1/crypto/send
/// This debits the user's USD wallet
pub async fn send_crypto(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(ip): ConnectInfo<SocketAddr>, // <-- Added IP
    TypedHeader(user_agent): TypedHeader<UserAgent>, // <-- Added User-Agent
    Json(payload): Json<CryptoSendPayload>,
) -> Result<impl IntoResponse, AppError> {
    
    let user_id = claims.sub;

    // 1. Verify PIN
    let pin_hash = sqlx::query!("SELECT pin_hash FROM users WHERE id = $1", user_id)
        .fetch_optional(&state.db_pool)
        .await.map_err(AppError::DatabaseError)?
        .and_then(|u| u.pin_hash)
        .ok_or(AppError::ProviderError("PIN_NOT_SET".to_string()))?;

    if !verify_value(payload.pin, pin_hash).await? {
        return Err(AppError::InvalidCredentials);
    }
    
    // 2. Sanitize Amount (convert 100.50 to 10050)
    let amount_minor = (payload.amount * 100.0).round() as i64;
    if amount_minor <= 0 {
        return Err(AppError::ProviderError("Invalid amount".to_string()));
    }
    
    // 3. Start ATOMIC Transaction
    let mut tx = state.db_pool.begin().await.map_err(AppError::DatabaseError)?;
    
    // 4. Get USD Wallet and Lock Row
    let wallet = sqlx::query!(
        "SELECT id, balance_minor FROM wallets WHERE user_id = $1 AND currency = 'USD' FOR UPDATE",
        user_id
    )
    .fetch_optional(&mut *tx)
    .await.map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError("USD wallet not found".to_storage()))?;
    
    // 5. Check Balance
    if wallet.balance_minor < amount_minor {
        tx.rollback().await.ok();
        return Err(AppError::ProviderError("Insufficient USD funds".to_string()));
    }
    
    // 6. Debit Wallet
    sqlx::query!(
        "UPDATE wallets SET balance_minor = balance_minor - $1 WHERE id = $2",
        amount_minor, wallet.id
    )
    .execute(&mut *tx)
    .await.map_err(AppError::DatabaseError)?;
    
    // 7. (MOCK) Call Provider to send crypto
    let provider_tx_id = state.crypto_provider_client
        .send_crypto(&payload.asset, &payload.network, payload.amount, &payload.to_address)
        .await
        .map_err(|e| {
            // If this fails, we must roll back!
            AppError::ProviderError(format!("Crypto send failed: {}", e))
        })?;
    
    let ip_address = ip.to_string();
    let user_agent_str = user_agent.to_string();
    let metadata = json!({
        "asset": payload.asset,
        "network": payload.network,
    });
        
    // --- 8. Log Transaction (UPDATED) ---
    sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title,
            counterparty, reference, metadata, ip_address, user_agent
        )
        VALUES ($1, $2, 'crypto_send', 'completed', $3, 'USD', $4, $5, $6, $7, $8, $9)
        "#,
        user_id,
        wallet.id,
        -amount_minor,
        format!("Send {}", payload.asset), // title
        payload.to_address, // counterparty
        provider_tx_id, // reference
        metadata, // metadata
        ip_address, // <-- Save IP
        user_agent_str // <-- Save User-Agent
    )
    .execute(&mut *tx)
    .await.map_err(AppError::DatabaseError)?;

    // 9. Commit
    tx.commit().await.map_err(AppError::DatabaseError)?;
    
    // 10. Log
    info!(
        amount = payload.amount,
        asset = %payload.asset,
        user_id = %user_id,
        "Processed crypto send"
    );
    
    Ok((StatusCode::OK, Json(json!({ "txId": provider_tx_id }))))
}