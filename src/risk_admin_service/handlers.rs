use crate::admin_auth_service::models::AdminClaims;
use crate::notification_service::service as notification_service; // <-- Need this
use crate::risk_admin_service::models::{FraudAlert, FundingApprovalPayload, HeldFundingEvent};
use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use tracing::info;
use uuid::Uuid;

/// Handler for GET /api/v1/admin/funding-events
/// Lists all funding events currently on HOLD
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn list_held_funding_events(
    State(state): State<AppState>,
    Extension(admin_claims): Extension<AdminClaims>,
) -> Result<impl IntoResponse, AppError> {
    let events = sqlx::query_as!(
        HeldFundingEvent,
        r#"
        SELECT
            fe.id as event_id,
            fe.transaction_id,
            fe.user_id,
            u.email as user_email,
            t.amount_minor,
            t.currency,
            fe.sender_name,
            fe.origin_bank,
            fe.name_match_score as "name_match_score!",
            fe.risk_score as "risk_score!",
            fe.decision,
            fe.created_at
        FROM funding_events fe
        JOIN transactions t ON fe.transaction_id = t.id
        JOIN users u ON fe.user_id = u.id
        WHERE fe.decision = 'HOLD'
        ORDER BY fe.created_at DESC
        LIMIT 100
        "#
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    info!(admin_id = %admin_claims.sub, "Viewed held funding events");
    Ok((StatusCode::OK, Json(events)))
}

/// Handler for GET /api/v1/admin/fraud-alerts
/// Lists all fraud alerts
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn list_fraud_alerts(
    State(state): State<AppState>,
    Extension(admin_claims): Extension<AdminClaims>,
) -> Result<impl IntoResponse, AppError> {
    let alerts = sqlx::query_as!(
        FraudAlert,
        r#"
        SELECT
            id, user_id, transaction_id, rule_triggered,
            risk_level, action_taken, metadata, created_at
        FROM fraud_alerts
        ORDER BY created_at DESC
        LIMIT 100
        "#
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    info!(admin_id = %admin_claims.sub, "Viewed fraud alerts");
    Ok((StatusCode::OK, Json(alerts)))
}

/// Handler for POST /api/v1/admin/funding-events/:id/approve
/// Manually approves a held deposit
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn approve_funding_event(
    State(state): State<AppState>,
    Extension(admin_claims): Extension<AdminClaims>,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<FundingApprovalPayload>,
) -> Result<impl IntoResponse, AppError> {
    // --- 1. Start ATOMIC Transaction ---
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    // 2. Get the funding event and associated transaction
    let event_info = sqlx::query!(
        r#"
        SELECT
            fe.user_id,
            t.id as transaction_id,
            t.wallet_id,
            t.amount_minor,
            t.currency
        FROM funding_events fe
        JOIN transactions t ON fe.transaction_id = t.id
        WHERE fe.id = $1 AND fe.decision = 'HOLD'
        FOR UPDATE
        "#,
        event_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError("Held event not found".to_string()))?;

    // 3. Update the funding event
    sqlx::query!(
        "UPDATE funding_events SET decision = $1, risk_score = 0 WHERE id = $2",
        "APPROVED_MANUAL",
        event_id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // 4. Update the transaction status
    sqlx::query!(
        "UPDATE transactions SET status = 'completed' WHERE id = $1",
        event_info.transaction_id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // 5. CRITICAL: Credit the user's wallet
    sqlx::query!(
        "UPDATE wallets SET balance_minor = balance_minor + $1 WHERE id = $2",
        event_info.amount_minor,
        event_info.wallet_id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // 6. (TODO) Log this to an admin audit log

    // 7. Commit
    tx.commit().await.map_err(AppError::DatabaseError)?;

    // 8. Send notification to user
    let title = "Deposit Approved".to_string();
    let body = format!(
        "Your deposit of {} {} has been approved and is now available.",
        (event_info.amount_minor as f64) / 100.0,
        event_info.currency
    );
    notification_service::create_notification(&state.db_pool, event_info.user_id, &title, &body)
        .await;

    info!(
        admin_id = %admin_claims.sub,
        event_id = %event_id,
        user_id = %event_info.user_id,
        reason = %payload.reason,
        "Manually approved held deposit"
    );

    Ok((StatusCode::OK, "Deposit approved successfully"))
}
