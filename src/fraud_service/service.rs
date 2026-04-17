use crate::error::AppError;
use crate::fraud_service::risk_models::{classify_spending, get_country_risk};
use crate::islamic_finance_service::rules::is_beneficiary_blocked;
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::Transaction;
use uuid::Uuid;

pub struct RiskAssessment {
    pub decision: String,
    pub status: String,
    pub risk_score: i32,
    pub rules_triggered: Vec<String>,
}

/// The main fraud check function.
#[allow(dead_code)]
pub async fn check_transaction_risk(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    _amount_minor: i64,
    ip_address: &str,
    user_agent: &str,
) -> Result<(), AppError> {
    // --- Rule 1: High-Velocity Check ---
    let time_window = Utc::now() - Duration::minutes(10);

    // FIXED: Added the actual SQL query string here
    let recent_tx_count = sqlx::query!(
        r#"
        SELECT COUNT(id) as "count!"
        FROM transactions
        WHERE user_id = $1 AND created_at > $2
        "#,
        user_id,
        time_window
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(AppError::DatabaseError)?;

    if recent_tx_count.count >= 3 {
        log_alert(
            tx,
            user_id,
            None,
            "HIGH_VELOCITY",
            "critical",
            "declined",
            json!({
                "count": recent_tx_count.count,
                "window_minutes": 10,
                "ip": ip_address,
                "userAgent": user_agent,
            }),
        )
        .await?;

        return Err(AppError::TransactionDeclined(
            "Too many recent transactions. Please try again later.".to_string(),
        ));
    }

    Ok(())
}

pub async fn check_payment_risk(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    _amount_minor: i64,
    channel: &str,
    beneficiary_name: &str,
    receiving_country: &str,
    _ip_address: &str,
    _user_agent: &str,
) -> Result<RiskAssessment, AppError> {
    let mut risk_score = 0;
    let mut rules_triggered = Vec::new();

    // 1. Spending Behavior
    let (category, score_adj) = classify_spending(channel, beneficiary_name);
    risk_score += score_adj;
    rules_triggered.push(format!("SPEND_CAT:{:?}", category));

    // 2. Cross-Border Risk
    if receiving_country != "NG" {
        let (zone, score_adj) = get_country_risk(receiving_country);
        risk_score += score_adj;
        rules_triggered.push(format!("X_BORDER:{:?}", zone));
    }

    // 3. High-Velocity Check (Re-used query logic)
    let time_window = Utc::now() - Duration::minutes(10);
    let recent_tx_count = sqlx::query!(
        r#"
        SELECT COUNT(id) as "count!"
        FROM transactions
        WHERE user_id = $1 AND created_at > $2
        "#,
        user_id,
        time_window
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(AppError::DatabaseError)?;

    if recent_tx_count.count >= 3 {
        risk_score += 50;
        rules_triggered.push("HIGH_VELOCITY".to_string());
    }

    // 4. Islamic Ethics Engine
    if is_beneficiary_blocked(beneficiary_name, channel) {
        risk_score = 100;
        rules_triggered.push("ISLAMIC_ETHICS:BLOCKED_BENEFICIARY".to_string());
    }

    // Final Decision
    let (decision, status) = if risk_score <= 20 {
        ("ALLOW".to_string(), "completed".to_string())
    } else if risk_score <= 70 {
        ("HOLD".to_string(), "pending".to_string())
    } else {
        ("BLOCK".to_string(), "failed".to_string())
    };

    Ok(RiskAssessment {
        decision,
        status,
        risk_score,
        rules_triggered,
    })
}

pub async fn log_alert(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    transaction_id: Option<Uuid>,
    rule_triggered: &str,
    risk_level: &str,
    action_taken: &str,
    metadata: serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        INSERT INTO fraud_alerts (
            user_id, transaction_id, rule_triggered, risk_level, action_taken, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        user_id,
        transaction_id,
        rule_triggered,
        risk_level,
        action_taken,
        metadata
    )
    .execute(&mut **tx)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(())
}
