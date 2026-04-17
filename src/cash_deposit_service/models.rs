use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashDepositConfig {
    pub supported_currencies: Vec<String>,
    pub meeting_methods: Vec<String>,
    pub cities: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDepositRequest {
    pub currency: String,
    pub amount: f64,
    pub city: String,
    pub location_detail: String,
    pub method: String,
    pub preferred_window: Option<String>,
    pub safety_confirmed: bool,
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CashDepositRow {
    pub id: Uuid,
    pub reference: String,
    pub user_id: Uuid,
    pub partner_org_id: Option<Uuid>,
    pub location_id: Option<Uuid>,
    pub currency: String,
    pub amount_minor: i64,
    pub requested_city: String,
    pub meeting_method: String,
    pub location_detail: String,
    pub preferred_window: Option<String>,
    pub safety_confirmed: bool,
    pub status: String,
    pub instructions: Option<String>,
    pub rejection_reason: Option<String>,
    pub credited_transaction_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositDetailResponse {
    pub reference: String,
    pub currency: String,
    pub amount_minor: i64,
    pub city: String,
    pub method: String,
    pub location_detail: String,
    pub preferred_window: Option<String>,
    pub safety_confirmed: bool,
    pub status: String,
    pub instructions: Option<String>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DepositHistoryItem {
    pub reference: String,
    pub currency: String,
    pub amount_minor: i64,
    pub city: String,
    pub method: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerDepositAcceptRequest {
    pub location_id: Option<Uuid>,
    pub instructions: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerDepositRejectRequest {
    pub reason: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerDepositActionResponse {
    pub reference: String,
    pub status: String,
}
