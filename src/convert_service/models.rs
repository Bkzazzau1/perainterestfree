use serde::{Deserialize, Serialize};

// Payload for POST /convert/execute
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertPayload {
    pub from_currency: String, // "USD", "NGN", "USDT"
    pub to_currency: String,   // "USD", "NGN", "USDT"

    // Amount of 'from_currency' in minor units
    pub amount_minor: i64,

    pub pin: String,
}

// Response for a successful conversion
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertResponse {
    pub from_currency: String,
    pub to_currency: String,
    pub from_amount_minor: i64,
    pub to_amount_minor: i64,
    pub rate: f64,
}
