#![allow(dead_code)]

use crate::islamic_finance_service::rules::is_mcc_blocked;
use crate::notification_service::service as notification_service;
use crate::{error::AppError, AppState};
use axum::extract::Query;
use axum::http::HeaderMap;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction}; // <-- Added Postgres
use tracing::{debug, info, warn};
use uuid::Uuid; // <-- UPDATED

// --- Imports for v2.0 Fraud Engine ---
use chrono::{DateTime, Duration, Utc};
// We don't need the fraud_service import here, as the logic is self-contained
// in check_funding_risk for this step.
// -------------------------------------

// --- STRUCTS ---

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrailsDeposit {
    pub account_number: String,
    pub amount_minor: i64,
    pub currency: String,
    pub reference: String,
    // --- NEW v2.0 FIELDS ---
    pub sender_name: Option<String>,
    pub origin_bank: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrailsCardAuth {
    pub provider_tx_id: String,   // The processor's transaction ID
    pub provider_card_id: String, // The card ID from Brails
    pub amount_minor: i64,        // The amount to charge (always positive)
    pub currency: String,
    pub merchant_name: String,
    pub mcc: String, // The Merchant Category Code
    pub is_foreign: bool,
}

/// A helper struct for our internal card data
struct CardData {
    id: Uuid,
    user_id: Uuid,
    balance_minor: i64,
    frozen: bool,
    allow_foreign: bool,
}

// --- (NEW) Helper Struct for Funding Check ---
struct UserFundingProfile {
    user_id: Uuid,
    wallet_id: Uuid,
    full_name: String,
    account_created_at: DateTime<Utc>,
}

// --- (NEW) Helper for Name Matching ---
/// Simple name matching. A real one would use fuzzy logic.
/// Returns (score, is_self_funding)
fn calculate_name_match(user_full_name: &str, sender_name_raw: Option<&String>) -> (f64, bool) {
    let sender_name = match sender_name_raw {
        Some(name) => name.trim().to_lowercase(),
        None => return (0.0, false), // No sender name is external and mismatched
    };

    let user_name = user_full_name.trim().to_lowercase();

    // Simple self-funding check
    if user_name == sender_name
        || user_name.contains(&sender_name)
        || sender_name.contains(&user_name)
    {
        (1.0, true) // 100% match, self-funding
    } else {
        (0.3, false) // Low match, external
    }
}

// --- (NEW) Funding Rules Engine (Section 4) ---
struct FundingDecision {
    decision: String,
    status: String,
    risk_score: i32,
    is_self_funding: bool,
    name_match_score: f64,
}

fn check_funding_risk(
    payload: &BrailsDeposit,
    user_profile: &UserFundingProfile,
) -> FundingDecision {
    let mut risk_score = 0;
    let mut decision = "ALLOW".to_string();

    let (name_match_score, is_self_funding) =
        calculate_name_match(&user_profile.full_name, payload.sender_name.as_ref());

    let is_external = !is_self_funding;
    let is_large_deposit = payload.amount_minor > 5_000_000; // ₦50,000
    let is_new_account = user_profile.account_created_at > (Utc::now() - Duration::days(7));

    // Rule: External NGN deposit > ₦50,000 → HOLD (Section 4 & 14)
    if is_external && is_large_deposit {
        decision = "HOLD".to_string();
        risk_score += 40;
    }

    // Rule: External funding with mismatched sender name → HOLD (Section 4)
    if is_external && name_match_score < 0.8 {
        decision = "HOLD".to_string();
        risk_score += 35; // (from Section 14: Non-self funding)
    }

    // Rule: Self-funding → ALLOW up to ₦10m (Section 4)
    if is_self_funding && payload.amount_minor > 1_000_000_000 {
        // ₦10m
        decision = "HOLD".to_string();
        risk_score += 50; // High value, even if self-funded
    }

    // Rule: Domestic deposits into new accounts (<7 days old) → HOLD (Section 4)
    if is_new_account && is_external {
        decision = "HOLD".to_string();
        risk_score += 20; // Add some risk
    }

    // Final status for the transaction log
    let status = if decision == "HOLD" {
        "pending".to_string()
    } else {
        "completed".to_string()
    };

    FundingDecision {
        decision,
        status,
        risk_score,
        is_self_funding,
        name_match_score,
    }
}

// --- HANDLERS ---

/// Handler for POST /webhooks/brails/deposit (REFACTORED)
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn brails_deposit(
    State(state): State<AppState>,
    Json(payload): Json<BrailsDeposit>,
) -> Result<impl IntoResponse, AppError> {
    // --- 1. Find the user and wallet for this deposit ---
    // We join 'virtual_accounts', 'wallets', 'users', and 'user_profiles'
    let user_profile = sqlx::query_as!(
        UserFundingProfile,
        r#"
        SELECT
            w.user_id,
            w.id as wallet_id,
            u.created_at as account_created_at,
            COALESCE(CONCAT(p.first_name, ' ', p.surname), '') as "full_name!: String"
        FROM wallets w
        JOIN virtual_accounts va ON w.user_id = va.user_id
        JOIN users u ON w.user_id = u.id
        JOIN user_profiles p ON w.user_id = p.user_id
        WHERE va.account_number = $1 AND w.currency = $2
        "#,
        payload.account_number,
        payload.currency
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError("Account not found".to_string()))?;

    // --- 2. Run the v2.0 Fraud Engine Rules (Section 4) ---
    let assessment = check_funding_risk(&payload, &user_profile);

    // --- 3. Start ATOMIC Database Transaction ---
    let mut tx: Transaction<Postgres> = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    let counterparty = payload
        .origin_bank
        .as_deref()
        .unwrap_or("External")
        .to_string();

    // --- 4. Update Wallet (or not) ---
    if assessment.decision == "ALLOW" {
        // Credit the user's wallet
        sqlx::query!(
            "UPDATE wallets SET balance_minor = balance_minor + $1 WHERE id = $2",
            payload.amount_minor,
            user_profile.wallet_id
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;
    }
    // If 'HOLD', we do *not* credit the wallet. The funds are in limbo.

    // --- 5. Create the Transaction Record (now includes status) ---
    let new_tx = sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title,
            counterparty, reference, metadata
        )
        VALUES ($1, $2, 'deposit', $3, $4, $5, $6, $7, $8, $9)
        RETURNING id
        "#,
        user_profile.user_id,
        user_profile.wallet_id,
        assessment.status, // 'pending' or 'completed'
        payload.amount_minor,
        payload.currency,
        "Incoming Deposit",
        counterparty,      // counterparty
        payload.reference, // reference
        json!({ "provider": "brails", "sender_name": payload.sender_name })
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    let sender_name = payload
        .sender_name
        .clone()
        .unwrap_or_else(|| "External".to_string());
    let origin_bank = payload
        .origin_bank
        .clone()
        .unwrap_or_else(|| "External".to_string());

    // --- 6. Log to new 'funding_events' table (Section 16) ---
    sqlx::query!(
        r#"
        INSERT INTO funding_events (
            user_id, transaction_id, sender_name, name_match_score,
            external_funding_flag, origin_bank, risk_score, decision
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        user_profile.user_id,
        new_tx.id,
        sender_name,
        assessment.name_match_score as f32,
        !assessment.is_self_funding,
        origin_bank,
        assessment.risk_score,
        assessment.decision
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- 7. Commit the Transaction ---
    tx.commit().await.map_err(AppError::DatabaseError)?;

    // --- 8. Send Notification ---
    let (title, body) = if assessment.decision == "ALLOW" {
        (
            "Deposit Received".to_string(),
            format!(
                "You just received {} {} in your {} wallet.",
                (payload.amount_minor as f64) / 100.0,
                payload.currency,
                payload.currency
            ),
        )
    } else {
        (
            "Deposit Under Review".to_string(),
            format!(
                "A deposit of {} {} is under review and will be available after verification.",
                (payload.amount_minor as f64) / 100.0,
                payload.currency
            ),
        )
    };

    notification_service::create_notification(&state.db_pool, user_profile.user_id, &title, &body)
        .await;

    // --- (NEW) SEND EMAIL ---
    let user_email = sqlx::query!(
        "SELECT email FROM users WHERE id = $1",
        user_profile.user_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map(|r| r.email)
    .unwrap_or_default(); // Fallback

    if !user_email.is_empty() {
        state
            .email_service
            .send_email(
                user_email, title, // Use the same title
                body,  // Use the same body
            )
            .await;
    }
    // ------------------------

    info!(
        amount = payload.amount_minor,
        currency = %payload.currency,
        user_id = %user_profile.user_id,
        decision = %assessment.decision,
        risk_score = assessment.risk_score,
        "Processed incoming deposit"
    );

    Ok(StatusCode::OK)
}

/// Handler for POST /webhooks/brails/card-auth
/// Approves or declines a real-time card transaction
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn brails_card_auth(
    State(state): State<AppState>,
    Json(payload): Json<BrailsCardAuth>,
) -> Result<impl IntoResponse, AppError> {
    // Start an ATOMIC database transaction
    let mut tx: Transaction<Postgres> = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    // 1. Get the card and LOCK THE ROW
    let card = sqlx::query_as!(
        CardData,
        r#"
        SELECT id, user_id, balance_minor, frozen, allow_foreign
        FROM cards
        WHERE provider_card_id = $1
        FOR UPDATE
        "#,
        payload.provider_card_id
    )
    .fetch_optional(&mut *tx) // Use 'tx'
    .await
    .map_err(AppError::DatabaseError)?;

    // --- Run all our checks ---
    let card = match card {
        Some(c) => c,
        None => {
            // --- ADDED ---
            debug!(provider_card_id = %payload.provider_card_id, "[DECLINED] Card auth: Card not found");
            // -------------
            return Err(AppError::TransactionDeclined("Card not found".to_string()));
        }
    };

    if card.frozen {
        // --- ADDED ---
        debug!(user_id = %card.user_id, card_id = %card.id, "[DECLINED] Card auth: Card is frozen");
        // -------------
        return Err(AppError::TransactionDeclined("Card is frozen".to_string()));
    }

    if payload.is_foreign && !card.allow_foreign {
        // --- ADDED ---
        debug!(user_id = %card.user_id, card_id = %card.id, "[DECLINED] Card auth: Foreign transactions disabled");
        // -------------
        return Err(AppError::TransactionDeclined(
            "Foreign transactions disabled".to_string(),
        ));
    }

    if is_mcc_blocked(&payload.mcc) {
        // --- ADDED ---
        debug!(user_id = %card.user_id, card_id = %card.id, mcc = %payload.mcc, "[DECLINED] Card auth: MCC blocked");
        // -------------
        return Err(AppError::TransactionDeclined(
            "Merchant category is blocked".to_string(),
        ));
    }

    if card.balance_minor < payload.amount_minor {
        // --- ADDED ---
        debug!(user_id = %card.user_id, card_id = %card.id, "[DECLINED] Card auth: Insufficient funds");
        // -------------
        return Err(AppError::TransactionDeclined(
            "Insufficient funds".to_string(),
        ));
    }

    // --- All checks passed. Approve the transaction. ---

    // 5. Debit the card's balance
    sqlx::query!(
        "UPDATE cards SET balance_minor = balance_minor - $1 WHERE id = $2",
        payload.amount_minor,
        card.id
    )
    .execute(&mut *tx) // Use 'tx'
    .await
    .map_err(AppError::DatabaseError)?;

    // 6. Log the transaction
    sqlx::query!(
        r#"
        INSERT INTO card_transactions (
            card_id, user_id, provider_tx_id, amount_minor,
            currency, merchant_name, mcc
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        card.id,
        card.user_id,
        payload.provider_tx_id,
        -payload.amount_minor, // Debits are negative
        payload.currency,
        payload.merchant_name,
        payload.mcc
    )
    .execute(&mut *tx) // Use 'tx'
    .await
    .map_err(AppError::DatabaseError)?;

    // 7. Commit the transaction
    tx.commit().await.map_err(AppError::DatabaseError)?;

    // Log with tracing
    info!(
        tx_id = %payload.provider_tx_id,
        card_id = %card.id,
        user_id = %card.user_id,
        amount = payload.amount_minor,
        mcc = %payload.mcc,
        "[APPROVED] Card auth"
    );

    // Return 200 OK. This tells Brails "Approved".
    Ok(StatusCode::OK)
}

// --- ADD THESE ---
#[derive(Deserialize, Debug)]
pub struct PayscribeWebhook {
    event_type: String,         // "bills.created", "bills.status"
    transaction_status: String, // "success", "fail"
    #[serde(rename = "ref")]
    reference: String, // This is *our* reference
    trans_id: String,           // This is Payscribe's reference
    remark: Option<String>,
}

/// Handler for POST /webhooks/payscribe/bills
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn payscribe_bill_status(
    State(state): State<AppState>,
    Json(payload): Json<PayscribeWebhook>,
) -> Result<impl IntoResponse, AppError> {
    // Only act on the final status update
    if payload.event_type != "bills.status" {
        return Ok(StatusCode::OK);
    }

    let new_status = match payload.transaction_status.as_str() {
        "success" => "completed",
        "fail" => "failed",
        _ => "pending", // "processing" or other
    };

    // Find the 'pending' transaction using our reference
    let result = sqlx::query!(
        "UPDATE transactions SET status = $1, metadata = metadata || $2 WHERE reference = $3 AND status = 'pending'",
        new_status,
        json!({ "provider_status": payload.remark, "provider_reference": payload.trans_id }),
        payload.reference
    )
    .execute(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;

    if result.rows_affected() == 0 {
        warn!(reference = %payload.reference, "Received webhook for unknown or completed transaction");
        // Return OK so Payscribe stops retrying
        return Ok(StatusCode::OK);
    }

    // (TODO) Send notification to user

    info!(reference = %payload.reference, new_status = %new_status, "Updated bill payment status from webhook");
    Ok(StatusCode::OK)
}

/// Handler for POST /api/v1/hooks/payscribe
/// Stores/logs the incoming webhook payload and headers, responds 200.
#[axum::debug_handler]
pub async fn payscribe_hook(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, AppError> {
    // For now, we log payload and headers; storage can be added when schema is ready.
    info!(?headers, ?payload, "Received Payscribe webhook");
    Ok((StatusCode::OK, Json(json!({"ok": true}))))
}

#[derive(Deserialize)]
pub struct PayscribeCallbackQuery {
    pub reference: Option<String>,
    pub status: Option<String>,
    pub message: Option<String>,
}

/// Handler for GET /payments/payscribe/callback
#[axum::debug_handler]
pub async fn payscribe_callback(
    Query(params): Query<PayscribeCallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    let response = json!({
        "ok": true,
        "reference": params.reference,
        "status": params.status,
        "message": params.message.unwrap_or_else(|| "Callback received".to_string()),
    });

    Ok((StatusCode::OK, Json(response)))
}
// -----------------
