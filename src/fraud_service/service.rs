use crate::{error::AppError, AppState};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::Transaction;
use uuid::Uuid;
// --- ADD THIS ---
use crate::fraud_service::risk_models::{get_country_risk, classify_spending};
use crate::islamic_finance_service::rules::is_beneficiary_blocked;
// ----------------

pub struct RiskAssessment {
    pub decision: String, // ALLOW, HOLD, BLOCK
    pub status: String,   // completed, pending
    pub risk_score: i32,
    pub rules_triggered: Vec<String>,
}

/// The main v2.0 fraud check for payments.
pub async fn check_payment_risk(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    amount_minor: i64,
    channel: &str,
    beneficiary_name: &str,
    receiving_country: &str,
    ip_address: &str,
    user_agent: &str,
) -> Result<RiskAssessment, AppError> {
    
    let mut risk_score = 0;
    let mut rules_triggered = Vec::new();

    // --- 1. Spending Behavior (Section 5 & 14) ---
    let (category, score_adj) = classify_spending(channel, beneficiary_name);
    risk_score += score_adj;
    rules_triggered.push(format!("SPEND_CAT:{:?}", category));

    // --- 2. Cross-Border Risk (Section 6 & 14) ---
    if receiving_country != "NG" {
        let (zone, score_adj) = get_country_risk(receiving_country);
        risk_score += score_adj;
        rules_triggered.push(format!("X_BORDER:{:?}", zone));
    }

    // --- 3. Pera-to-Pera Risk (Section 7 & 14) ---
    if category == crate::fraud_service::risk_models::SpendingCategory::PeraToPera {
        // (MOCK) We'd look up the receiver's risk. Assume clean for now.
        let receiver_risk = "clean"; 
        if receiver_risk == "clean" {
            risk_score += -20; // Pera-to-Pera clean
        } else {
            risk_score += 30; // Pera-to-Pera risk
        }
        rules_triggered.push(format!("P2P_RISK:{}", receiver_risk));
    }

    // --- 4. High-Velocity Check (from our old rules) ---
    let time_window = Utc::now() - Duration::minutes(10);
    let recent_tx_count = sqlx::query!(/* ... */)
        .fetch_one(&mut **tx).await.map_err(AppError::DatabaseError)?;
    
    if recent_tx_count.count >= 3 {
        risk_score += 50; // High velocity
        rules_triggered.push("HIGH_VELOCITY".to_string());
    }

    // --- 5. Geo & Device Rules (Section 9 & 10) ---
    if user_agent.contains("Emulator") {
        risk_score += 50; // Section 10
        rules_triggered.push("DEVICE:EMULATOR".to_string());
    }
    // (MOCK) Check for VPN/TOR
    if ip_address.starts_with("10.") { // A naive private IP check
        risk_score += 25; // Section 10
        rules_triggered.push("NET:VPN_DETECTED".to_string());
    }

    // --- 6. Islamic Ethics Engine (Section 12) ---
    if is_beneficiary_blocked(beneficiary_name, channel) {
        risk_score = 100; // Auto-block
        rules_triggered.push("ISLAMIC_ETHICS:BLOCKED_BENEFICIARY".to_string());
    }

    // --- 7. Final Decision (Section 13) ---
    let (decision, status) = if risk_score <= 20 {
        ("ALLOW".to_string(), "completed".to_string())
    } else if risk_score <= 70 {
        ("HOLD".to_string(), "pending".to_string()) // HOLD for review
    } else {
        ("BLOCK".to_string(), "failed".to_string()) // FREEZE/BLOCK
    };

    Ok(RiskAssessment {
        decision,
        status,
        risk_score,
        rules_triggered,
    })
}

// ... (log_alert function is unchanged)