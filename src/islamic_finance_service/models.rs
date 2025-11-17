use serde::{Deserialize, Serialize};

// Matches 'zakat_service.dart'
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZakatRates {
    pub gold_per_gram: f64,
    pub silver_per_gram: f64,
}

// Payload for POST /islamic/pay-zakat
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayZakatPayload {
    // Amount in NGN major units (e.g., 5000.50)
    pub amount: f64,
    pub pin: String,
    // The ID of the charity we are paying
    pub beneficiary_id: String, 
}