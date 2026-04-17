use crate::auth::{jwt::Claims, security::verify_value};
use crate::fraud_service::service as fraud_service;
use crate::payment_service::models::{TransferPayload, TransferResponse};
use crate::{error::AppError, AppState};
use axum::{
    extract::{ConnectInfo, State}, // <-- Added ConnectInfo
    http::StatusCode,
    response::IntoResponse,
    Extension,
    Json,
};
use axum_extra::{headers::UserAgent, TypedHeader}; // <-- Updated
use serde_json::json;
use std::net::SocketAddr; // <-- Added
use tracing::{debug, info}; // <-- UPDATED
use uuid::Uuid; // <-- Use the service

/// Handler for GET /api/v1/payments/country-matrix
#[axum::debug_handler]
pub async fn get_country_matrix() -> Result<impl IntoResponse, AppError> {
    Ok((
        StatusCode::OK,
        Json(json!({
            "Kenya": ["bank", "mobile_money"],
            "Ghana": ["bank", "mobile_money"],
            "Uganda": ["bank", "mobile_money"],
            "Tanzania": ["bank", "mobile_money"],
            "Zambia": ["bank", "mobile_money"],
            "Malawi": ["bank", "mobile_money"],
            "Senegal": ["bank", "mobile_money"],
            "Ivory Coast": ["bank", "mobile_money"],
            "Cameroon": ["bank", "mobile_money"],
            "Guinea": ["bank", "mobile_money"],
            "Burkina Faso": ["bank", "mobile_money"],
            "Mali": ["bank", "mobile_money"],
            "Togo": ["bank", "mobile_money"],
            "Benin": ["bank", "mobile_money"],
            "Niger Republic": ["bank", "mobile_money"],
            "Nigeria": ["bank"],
            "United States": ["bank"],
            "United Kingdom": ["bank"],
            "France": ["bank"],
            "Spain": ["bank"],
            "Italy": ["bank"],
            "Australia": ["bank"],
            "Singapore": ["bank"],
            "United Arab Emirates": ["bank"],
            "China": ["bank"],
            "DR Congo": ["bank"]
        })),
    ))
}

/// Handler for POST /api/v1/payments/transfer
/// This is the core P2P / Payout endpoint
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn perform_transfer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(ip): ConnectInfo<SocketAddr>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    Json(payload): Json<TransferPayload>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;
    let ip_address = ip.to_string();
    let user_agent_str = user_agent.to_string();

    // --- 1. Security: Verify PIN ---
    let user = sqlx::query!("SELECT pin_hash FROM users WHERE id = $1", user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::Unauthorized)?; // Should not happen

    let pin_hash = user
        .pin_hash
        .ok_or(AppError::ProviderError("PIN_NOT_SET".to_string()))?;

    let pin_valid = verify_value(payload.pin, pin_hash).await?;
    if !pin_valid {
        // --- ADDED ---
        debug!(user_id = %user_id, "P2P transfer failed: incorrect PIN");
        // -------------
        return Err(AppError::InvalidCredentials); // "Invalid credentials"
    }

    // --- 2. Sanitize Amount & Get Counterparty ---
    let amount_minor = (payload.amount * 100.0).round() as i64;
    if amount_minor <= 0 {
        return Err(AppError::ProviderError("Invalid amount".to_string()));
    }

    let counterparty = payload
        .beneficiary
        .get("account_name")
        .or_else(|| payload.beneficiary.get("full_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    // --- 3. Start ATOMIC Transaction ---
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    // --- 4. (NEW) Call v2.0 Fraud Engine ---
    let assessment = fraud_service::check_payment_risk(
        &mut tx,
        user_id,
        amount_minor,
        &payload.channel,
        &counterparty,
        &payload.country,
        &ip_address,
        &user_agent_str,
    )
    .await?;

    // --- 5. Check Decision ---
    if assessment.decision == "BLOCK" {
        // --- ADDED --- (Log was already here, but for clarity)
        info!(user_id = %user_id, score = assessment.risk_score, "P2P transfer blocked by fraud check");
        // -------------
        // Log this critical event
        fraud_service::log_alert(
            &mut tx,
            user_id,
            None,
            "BLOCK_DECISION",
            "critical",
            "declined",
            json!({"rules": assessment.rules_triggered}),
        )
        .await?;
        tx.commit().await.ok(); // Commit the log, but decline the payment
        return Err(AppError::TransactionDeclined(
            "Transaction blocked by risk policy".to_string(),
        ));
    }

    // --- 6. Get Wallet and Lock Row ---
    let wallet = sqlx::query!(
        r#"
        SELECT id, balance_minor
        FROM wallets
        WHERE user_id = $1 AND currency = $2
        FOR UPDATE
        "#,
        user_id,
        payload.source_currency
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError("Wallet not found".to_string()))?;

    // --- 7. Check Balance ---
    if wallet.balance_minor < amount_minor {
        tx.rollback().await.ok(); // Rollback is best-effort
                                  // --- ADDED ---
        debug!(user_id = %user_id, currency = %payload.source_currency, "P2P transfer failed: insufficient funds");
        // -------------
        return Err(AppError::ProviderError("Insufficient funds".to_string()));
    }

    // --- 8. Debit Wallet ---
    sqlx::query!(
        r#"
        UPDATE wallets
        SET balance_minor = balance_minor - $1
        WHERE id = $2
        "#,
        amount_minor,
        wallet.id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- 9. Create Transaction Record ---
    let new_tx = sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title,
            counterparty, reference, metadata, ip_address, user_agent
        )
        VALUES ($1, $2, 'payout', $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING id
        "#,
        user_id,
        wallet.id,
        assessment.status, // 'pending' or 'completed'
        -amount_minor, // Debits are negative
        payload.source_currency,
        format!("Send to {}", payload.country), // title
        counterparty, // counterparty
        Uuid::new_v4().to_string(), // Mock reference
        json!({ "rules": assessment.rules_triggered, "score": assessment.risk_score, "note": payload.note }),
        ip_address,
        user_agent_str
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- 10. (NEW) Update Behavior Profile (Section 16) ---
    sqlx::query!(
        r#"
        INSERT INTO behavior_profiles (user_id, velocity_24h_count, velocity_7d_count, velocity_24h_value_minor, velocity_7d_value_minor, updated_at)
        VALUES ($1, 1, 1, $2, $2, NOW())
        ON CONFLICT (user_id) DO UPDATE SET
            velocity_24h_count = behavior_profiles.velocity_24h_count + 1,
            velocity_7d_count = behavior_profiles.velocity_7d_count + 1,
            velocity_24h_value_minor = behavior_profiles.velocity_24h_value_minor + $2,
            velocity_7d_value_minor = behavior_profiles.velocity_7d_value_minor + $2,
            updated_at = NOW()
        "#,
        user_id,
        amount_minor
    )
    .execute(&mut *tx)
    .await.map_err(AppError::DatabaseError)?;

    // --- 11. Commit ---
    tx.commit().await.map_err(AppError::DatabaseError)?;

    let response = TransferResponse {
        id: new_tx.id,
        status: assessment.status, // Return 'pending' or 'completed'
        amount: payload.amount,
        channel: payload.channel,
        country: payload.country,
        source_currency: payload.source_currency,
    };

    info!(
        user_id = %user_id,
        tx_id = %new_tx.id,
        decision = %assessment.decision,
        score = assessment.risk_score,
        "Processed P2P transfer"
    );

    Ok((StatusCode::OK, Json(response)))
}
