use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    pub from_asset: String,
    pub to_asset: String,
    pub rate: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoSendPayload {
    pub asset: String,   // "USDT"
    pub network: String, // "TRC20"
    pub to_address: String,
    pub amount: f64, // Major units (e.g., 100.50)
    #[allow(dead_code)]
    pub memo_tag: Option<String>,
    pub pin: String,
}

// --- 'ConvertPayload' and 'ConvertResponse' are no longer needed here ---
