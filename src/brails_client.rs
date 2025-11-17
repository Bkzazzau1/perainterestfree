use serde::{Deserialize, Serialize};
use reqwest::Client; // <-- Added
use std::collections::HashMap; // <-- Added

/// The response we expect from Brails for virtual accounts
#[derive(Debug, Serialize, Deserialize)]
pub struct BrailsAccount {
    pub bank_name: String,
    pub account_number: String,
    pub account_name: String, // The name verified by BVN
}

// --- NEW STRUCT FOR BRAILS API RESPONSE ---
#[derive(Debug, Deserialize)]
struct BrailsRateResponse {
    rates: HashMap<String, f64>,
}
// ------------------------------------------

/// Our mock Brails client
#[derive(Clone)]
pub struct BrailsClient {
    http_client: Client, // <-- Add a real HTTP client
}

impl BrailsClient {
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }

    /// Fetches the live exchange rate from Brails
    pub async fn get_exchange_rate(
        &self,
        api_key: &str,
        from: &str,
        to: &str,
    ) -> Result<f64, String> {
        let url = "https://api.brails.com/v1/exchange-rates";
        
        let response = self.http_client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .query(&[("from", from), ("to", to)])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!("Brails API error: {}", response.status()));
        }

        let body = response
            .json::<BrailsRateResponse>()
            .await
            .map_err(|e| e.to_string())?;
        
        // Brails returns a map, e.g., {"NGN": 1450.0}
        body.rates.get(to)
            .map(|rate| *rate)
            .ok_or_else(|| "Target currency not found in Brails response".to_string())
    }

    /// Mock function to "create" a virtual account
    /// In a real app, this would be 'async' and make an HTTP request.
    pub fn create_virtual_account(
        &self,
        first_name: &str,
        surname: &str,
        bvn: &str,
    ) -> Result<BrailsAccount, String> {
        // --- Mock Logic ---
        // 1. Simulate a check on the BVN
        if bvn != "12345678901" {
            // BVN is valid, but doesn't match the name
            if bvn == "11111111111" {
                 return Err("BVN_NAME_MISMATCH".to_string());
            }
            // A truly invalid BVN
            return Err("INVALID_BVN".to_string());
        }

        // 2. Simulate success
        // Brails confirms the name from the BVN
        let official_name = format!("{} {}", surname, first_name).to_uppercase();

        Ok(BrailsAccount {
            bank_name: "Pera Bank (via Brails)".to_string(),
            account_number: "1234567890".to_string(),
            account_name: official_name,
        })
        // ------------------
    }
}