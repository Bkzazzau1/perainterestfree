use serde::{Deserialize, Serialize}; // <-- Added Deserialize
use uuid::Uuid;
use chrono::{DateTime, Utc};

// This matches the 'WalletSummary' class and the JSON in 'wallets_controller.dart'
#[derive(Serialize)]
#[serde(rename_all = "camelCase")] // Outputs JSON as camelCase
pub struct WalletSummary {
    pub name: String,
    pub account_number: String,
    pub currency: String,
    pub balance_minor: i64,
}

// --- ADD THIS STRUCT ---
/// Query parameters for GET /wallets/transactions
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryQuery {
    pub q: Option<String>, // Search query
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    // We could add offset/limit for pagination here too
}
// -----------------------

// --- UPDATE THIS STRUCT ---
// Matches the 'Tx' model used in 'receipt_view.dart'
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub id: Uuid,
    pub title: String,
    #[serde(rename = "amountKobo")]
    pub amount_minor: i64,
    #[serde(rename = "createdAt")]
    pub at: DateTime<Utc>,
    pub currency: String,
    pub status: String,
    // Add new searchable fields
    pub counterparty: Option<String>,
    pub reference: Option<String>,
    #[serde(rename = "channel")]
    pub transaction_type: String, // Renamed from 'type'
}