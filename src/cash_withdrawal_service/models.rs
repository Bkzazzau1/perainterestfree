use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashWithdrawalConfig {
    pub supported_currencies: Vec<String>,
    pub methods: Vec<String>,
    pub supports_delivery: bool,
    pub pickup_expiry_hours: i64,
    pub cities: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    pub currency: String,
    pub method: String,
    pub amount: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    pub fee_minor: i64,
    pub total_debit_minor: i64,
    pub expiry_hours: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWithdrawalRequest {
    pub currency: String,
    pub method: String,
    pub amount: f64,
    pub city: Option<String>,
    pub location_detail: Option<String>,
    pub location_id: Option<Uuid>,
    pub delivery_address: Option<Value>,
    pub pin: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerReadyRequest {
    pub location_id: Option<Uuid>,
    pub expiry_hours: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerConfirmRequest {
    pub pickup_code: Option<String>,
    pub delivered: Option<bool>,
    pub proof: Option<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerReadyResponse {
    pub reference: String,
    pub pickup_code: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerActionResponse {
    pub reference: String,
    pub status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickupInstructions {
    pub location_id: Option<Uuid>,
    pub city: Option<String>,
    pub address: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CashWithdrawalRow {
    pub id: Uuid,
    pub reference: String,
    pub user_id: Uuid,
    pub partner_org_id: Option<Uuid>,
    pub location_id: Option<Uuid>,
    pub currency: String,
    pub method: String,
    pub amount_minor: i64,
    pub fee_minor: i64,
    pub total_debit_minor: i64,
    pub requested_city: Option<String>,
    pub location_detail: Option<String>,
    pub status: String,
    pub pickup_code_expires_at: Option<DateTime<Utc>>,
    pub delivery_address: Option<Value>,
    pub failed_attempts: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalDetailResponse {
    pub reference: String,
    pub currency: String,
    pub method: String,
    pub amount_minor: i64,
    pub fee_minor: i64,
    pub total_debit_minor: i64,
    pub city: Option<String>,
    pub location_detail: Option<String>,
    pub status: String,
    pub pickup_instructions: Option<PickupInstructions>,
    pub delivery_address: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalHistoryItem {
    pub reference: String,
    pub currency: String,
    pub method: String,
    pub amount_minor: i64,
    pub fee_minor: i64,
    pub total_debit_minor: i64,
    pub city: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerListQuery {
    pub status: Option<String>,
    pub city: Option<String>,
    pub currency: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct PartnerContext {
    pub user_id: Uuid,
    pub partner_org_id: Uuid,
}
