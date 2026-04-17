use crate::auth::{jwt::Claims, security::verify_value};
use crate::convert_service::models::{ConvertPayload, ConvertResponse};
use crate::{error::AppError, AppState};
use axum::{
    extract::{ConnectInfo, State}, // <-- Added ConnectInfo
    http::StatusCode,
    response::IntoResponse,
    Extension,
    Json,
};
use axum_extra::headers::UserAgent; // <-- Added
use axum_extra::TypedHeader; // <-- Added
use std::net::SocketAddr; // <-- Added
use tracing::{debug, info};
use uuid::Uuid; // <-- UPDATED
                // --- ADDED ---
use crate::admin_settings_service::service::get_all_settings;
// ---------------

/// Helper to map currencies to wallet keys.
/// As per 'convert_view.dart', USDT uses the 'USD' wallet.
fn currency_to_wallet_key(currency: &str) -> &str {
    match currency {
        "USDT" => "USD",
        _ => currency,
    }
}

/// Handler for POST /api/v1/convert/execute
/// Atomically swaps one currency for another
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn execute_conversion(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(ip): ConnectInfo<SocketAddr>, // <-- Added IP
    TypedHeader(user_agent): TypedHeader<UserAgent>, // <-- Added User-Agent
    Json(payload): Json<ConvertPayload>,
) -> Result<impl IntoResponse, AppError> {
    // Destructure to avoid moving fields multiple times
    let ConvertPayload {
        from_currency,
        to_currency,
        amount_minor,
        pin,
    } = payload;

    let user_id = claims.sub;

    // 1. Validate Payload
    if amount_minor <= 0 {
        return Err(AppError::ProviderError("Invalid amount".to_string()));
    }
    if from_currency == to_currency {
        return Err(AppError::ProviderError(
            "Cannot convert to same currency".to_string(),
        ));
    }

    // 2. Verify PIN
    let pin_hash = sqlx::query!("SELECT pin_hash FROM users WHERE id = $1", user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?
        .and_then(|u| u.pin_hash)
        .ok_or(AppError::ProviderError("PIN_NOT_SET".to_string()))?;

    if !verify_value(pin, pin_hash).await? {
        // --- ADDED ---
        debug!(user_id = %user_id, "Currency conversion failed: incorrect PIN");
        // -------------
        return Err(AppError::InvalidCredentials);
    }

    // --- 3. Get Live Rates & Admin Settings (REFACTORED) ---
    let settings = get_all_settings(&state.db_pool).await?;
    let api_key = settings
        .get("brails_api_key")
        .ok_or(AppError::ProviderError(
            "Brails API key not set".to_string(),
        ))?;
    let markup: f64 = settings
        .get("fx_rate_usd_ngn_markup")
        .unwrap_or(&"0.0".to_string())
        .parse()
        .unwrap_or(0.0);

    let from_key = currency_to_wallet_key(&from_currency);
    let to_key = currency_to_wallet_key(&to_currency);

    // --- 4. Get Live Rate from Brails (not mock) ---
    let live_rate = if from_key == to_key {
        1.0
    } else {
        state
            .brails_client
            .get_exchange_rate(api_key, from_key, to_key)
            .await
            .map_err(AppError::ProviderError)?
    };

    // 5. Apply Admin Markup
    // We only apply markup on NGN pairs
    let final_rate = if from_key == "NGN" || to_key == "NGN" {
        live_rate + markup
    } else {
        live_rate
    };

    // 6. Calculate amounts
    let from_amount_minor = amount_minor;
    let to_amount_minor = ((from_amount_minor as f64 / 100.0) * final_rate * 100.0).round() as i64;

    // 7. Start ATOMIC Transaction
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    // 8. Get and Lock FROM wallet
    let from_wallet = sqlx::query!(
        "SELECT id, balance_minor FROM wallets WHERE user_id = $1 AND currency = $2 FOR UPDATE",
        user_id,
        from_key
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError(format!(
        "{} wallet not found",
        from_key
    )))?;

    // 9. Check balance
    if from_wallet.balance_minor < amount_minor {
        tx.rollback().await.ok();
        // --- ADDED ---
        debug!(user_id = %user_id, currency = %from_key, "Currency conversion failed: insufficient funds");
        // -------------
        return Err(AppError::ProviderError("Insufficient funds".to_string()));
    }

    // 10. Get and Lock TO wallet
    let to_wallet = sqlx::query!(
        "SELECT id FROM wallets WHERE user_id = $1 AND currency = $2 FOR UPDATE",
        user_id,
        to_key
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError(format!(
        "{} wallet not found",
        to_key
    )))?;

    // 11. Debit FROM wallet
    sqlx::query!(
        "UPDATE wallets SET balance_minor = balance_minor - $1 WHERE id = $2",
        amount_minor,
        from_wallet.id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // 12. Credit TO wallet
    sqlx::query!(
        "UPDATE wallets SET balance_minor = balance_minor + $1 WHERE id = $2",
        to_amount_minor,
        to_wallet.id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- Get shared transaction info ---
    let conversion_id = Uuid::new_v4().to_string();
    let ip_address = ip.to_string();
    let user_agent_str = user_agent.to_string();

    // --- 13. Log Debit Transaction ---
    sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title,
            counterparty, reference, ip_address, user_agent
        )
        VALUES ($1, $2, 'convert', 'completed', $3, $4, $5, 'Pera Convert', $6, $7, $8)
        "#,
        user_id,
        from_wallet.id,
        -amount_minor,
        from_key,
        format!("Convert to {}", to_currency.as_str()), // title
        conversion_id.clone(),                          // reference
        ip_address.clone(),                             // ip_address
        user_agent_str.clone()                          // user_agent
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- 14. Log Credit Transaction ---
    sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title,
            counterparty, reference, ip_address, user_agent
        )
        VALUES ($1, $2, 'convert', 'completed', $3, $4, $5, 'Pera Convert', $6, $7, $8)
        "#,
        user_id,
        to_wallet.id,
        to_amount_minor,
        to_key,
        format!("Convert from {}", from_currency.as_str()), // title
        conversion_id,                                      // reference
        ip_address,                                         // ip_address
        user_agent_str                                      // user_agent
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // 15. Commit
    tx.commit().await.map_err(AppError::DatabaseError)?;

    // 16. Log
    info!(
        from = %from_currency,
        to = %to_currency,
        amount = amount_minor,
        user_id = %user_id,
        rate = final_rate,
        "Processed currency conversion"
    );

    let response = ConvertResponse {
        from_currency,
        to_currency,
        from_amount_minor: amount_minor,
        to_amount_minor,
        rate: final_rate,
    };

    Ok((StatusCode::OK, Json(response)))
}
