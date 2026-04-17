use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json; // <-- Added this import, required for json!()
use uuid::Uuid;

// --- NEW: Struct for Brails Send Request (Source 73) ---
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrailsSendPayload {
    pub amount: i64, // Amount in cents (minor units)
    pub address: String,
    pub chain: String,
    pub reference: String, // Our unique transaction ID
    pub description: String,
    pub customer_email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
}

// --- NEW: Struct for Brails Address Response (Source 146) ---
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrailsAddressData {
    #[allow(dead_code)]
    pub id: String,
    pub address: String,
    #[allow(dead_code)]
    pub customer_email: Option<String>,
    #[allow(dead_code)]
    pub network: String,
    #[allow(dead_code)]
    pub address_type: String, // "usdt" or "usdc"
}

#[derive(Deserialize)]
struct BrailsApiResponse<T> {
    status: bool,
    message: String,
    data: T,
}

// ---------------------------------------------------

#[derive(Clone)]
pub struct CryptoProviderClient {
    http_client: Client,
    base_url: String, // e.g., "https://sandboxapi.onbrails.com/api/v1"
}

impl CryptoProviderClient {
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
            // We'll get this from .env in a real setup
            base_url: "https://sandboxapi.onbrails.com/api/v1".to_string(),
        }
    }

    /// (Refactored) POST /wallets/collections/depositAddress/{stableCoin}/{chain} (Source 120)
    pub async fn get_deposit_address(
        &self,
        api_key: &str,
        asset: &str, // "usdt" or "usdc"
        chain: &str, // "tron", "bsc", etc.
        customer_email: &str,
    ) -> Result<BrailsAddressData, String> {
        let url = format!(
            "{}/wallets/collections/depositAddress/{}/{}",
            self.base_url, asset, chain
        );

        let payload = json!({ "customerEmail": customer_email });

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!(
                "Brails API error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let body = response
            .json::<BrailsApiResponse<BrailsAddressData>>()
            .await
            .map_err(|e| e.to_string())?;

        if !body.status {
            return Err(body.message);
        }

        Ok(body.data)
    }

    /// (Refactored) POST /wallets/send/{stablecoin} (Source 66, 67)
    pub async fn send_stablecoin(
        &self,
        api_key: &str,
        asset: &str, // "usdt" or "usdc"
        payload: BrailsSendPayload,
    ) -> Result<String, String> {
        let url = format!("{}/wallets/send/{}", self.base_url, asset);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!(
                "Brails API error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        // (Mock) Extract provider TX ID from response (Source 107)
        // In a real app, we'd deserialize this properly.
        Ok(Uuid::new_v4().to_string())
    }

    /// (Unchanged) Get a conversion quote
    pub async fn get_quote(&self, from: &str, to: &str) -> Result<f64, String> {
        // This remains a mock, as per the file context
        if from == "USD" && to == "NGN" {
            Ok(1495.50) // Mock rate
        } else {
            Err("Invalid pair".to_string())
        }
    }
}
