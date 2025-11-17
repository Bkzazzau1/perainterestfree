use serde::{Deserialize, Serialize};
use serde_json::Value; // For flexible beneficiary
use uuid::Uuid;

// This struct perfectly matches the 'buildPayload' map
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TransferPayload {
    pub source_currency: String, // "NGN", "USD", etc.
    pub country: String,
    pub channel: String, // "bank", "mobile_money"
    pub amount: f64, // The major unit amount (e.g., 100.50)
    pub note: String,
    pub pin: String, // Plain-text PIN from client
    pub beneficiary: Value, // We'll parse this as JSON
}

// The response we send back, matching the 'receipt'
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferResponse {
    pub id: Uuid, // Our internal transaction ID
    pub status: String,
    pub amount: f64,
    pub channel: String,
    pub country: String,
    pub source_currency: String,
}