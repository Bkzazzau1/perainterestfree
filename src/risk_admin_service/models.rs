use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json::Value;

// For GET /admin/funding-events
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeldFundingEvent {
    pub event_id: Uuid,
    pub transaction_id: Uuid,
    pub user_id: Uuid,
    pub user_email: String,
    pub amount_minor: i64,
    pub currency: String,
    pub sender_name: Option<String>,
    pub origin_bank: Option<String>,
    pub name_match_score: f64,
    pub risk_score: i32,
    pub decision: String,
    pub created_at: DateTime<Utc>,
}

// For GET /admin/fraud-alerts
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FraudAlert {
    pub id: Uuid,
    pub user_id: Uuid,
    pub transaction_id: Option<Uuid>,
    pub rule_triggered: String,
    pub risk_level: String,
    pub action_taken: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

// For POST /admin/funding-events/:id/approve
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingApprovalPayload {
    pub reason: String,
}