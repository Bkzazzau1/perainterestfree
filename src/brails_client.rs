use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// --- STRUCT FOR BRAILS API RESPONSE ---
#[derive(Debug, Deserialize)]
struct BrailsRateResponse {
    rates: HashMap<String, f64>,
}
// ------------------------------------------

// --- STRUCTS FOR CUSTOMERS ---
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrailsCreateCustomerPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrailsUpdateCustomerPayload {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrailsCustomer {
    pub id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub country_code: Option<String>,
    #[serde(rename = "customerId")]
    pub customer_id: Option<String>,
    pub blacklist: Option<bool>,
    #[serde(flatten)]
    pub extra: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrailsCustomerList {
    pub customers: Vec<BrailsCustomer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

#[derive(Deserialize)]
struct BrailsCustomerEnvelope {
    status: bool,
    message: Option<String>,
    data: BrailsCustomer,
}

#[derive(Deserialize)]
struct BrailsCustomerListEnvelope {
    status: bool,
    message: Option<String>,
    data: BrailsCustomerList,
}
// ------------------------------------------

// --- STRUCTS FOR CARD USER REG (Source 335) ---
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrailsRegisterUserPayload {
    pub customer_email: String,
    #[serde(rename = "idNumber")]
    pub id_number: String,
    #[serde(rename = "idType")]
    pub id_type: String,
    pub first_name: String,
    pub last_name: String,
    pub phone_number: String,
    pub city: String,
    pub state: String,
    pub country: String, // e.g., "NG"
    pub bvn: String,
    pub user_photo: String, // base64 encoded string
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrailsRegisterUserData {
    #[serde(rename = "userId")]
    pub user_id: String, // This is the Brails ID
    #[allow(dead_code)]
    pub customer_email: String,
    #[serde(rename = "kycStatus")]
    pub kyc_status: String, // e.g., "PENDING"
}

#[derive(Deserialize)]
struct BrailsRegisterUserResponse {
    status: bool,
    data: BrailsRegisterUserData,
}
// -----------------------------------

// --- STRUCT FOR VIRTUAL ACCOUNT (Source 351) ---
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrailsVirtualAccountPayload {
    pub first_name: String,
    pub last_name: String,
    pub bvn: String,
    pub date_of_birth: String, // "YYYY-MM-DD"
    pub customer_email: String,
    pub reference: String, // Our internal user_id
    pub bank: String,      // e.g., "providus"
    pub phone_number: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrailsVirtualAccountData {
    #[allow(dead_code)]
    pub id: String,
    pub bank: String,
    pub account_name: String,
    pub account_number: String,
    pub status: String,
}

#[derive(Deserialize)]
struct BrailsVirtualAccountResponse {
    status: bool,
    data: BrailsVirtualAccountData,
}
// ------------------------------------------

/// Our Brails client
#[derive(Clone)]
pub struct BrailsClient {
    http_client: Client,
    base_url: String,
}

impl BrailsClient {
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
            // Assuming the same sandbox base URL as other services
            base_url: "https://sandboxapi.onbrails.com/api/v1".to_string(),
        }
    }

    /// Fetches the live exchange rate from Brails
    pub async fn get_exchange_rate(
        &self,
        api_key: &str,
        from: &str,
        to: &str,
    ) -> Result<f64, String> {
        // Note: This API endpoint seems different from the sandbox one.
        let url = "https://api.brails.com/v1/exchange-rates";

        let response = self
            .http_client
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
        body.rates
            .get(to)
            .map(|rate| *rate)
            .ok_or_else(|| "Target currency not found in Brails response".to_string())
    }

    /// Creates a customer in Brails
    pub async fn create_customer(
        &self,
        api_key: &str,
        payload: BrailsCreateCustomerPayload,
    ) -> Result<BrailsCustomer, String> {
        let url = format!("{}/customers", self.base_url);

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
            .json::<BrailsCustomerEnvelope>()
            .await
            .map_err(|e| e.to_string())?;

        if !body.status {
            return Err(body
                .message
                .unwrap_or_else(|| "Brails returned failure".to_string()));
        }

        Ok(body.data)
    }

    /// Retrieves a customer by id
    pub async fn get_customer(
        &self,
        api_key: &str,
        customer_id: &str,
    ) -> Result<BrailsCustomer, String> {
        let url = format!("{}/customers/{}", self.base_url, customer_id);

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
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
            .json::<BrailsCustomerEnvelope>()
            .await
            .map_err(|e| e.to_string())?;

        if !body.status {
            return Err(body
                .message
                .unwrap_or_else(|| "Brails returned failure".to_string()));
        }

        Ok(body.data)
    }

    /// Updates a customer
    pub async fn update_customer(
        &self,
        api_key: &str,
        customer_id: &str,
        payload: BrailsUpdateCustomerPayload,
    ) -> Result<BrailsCustomer, String> {
        let url = format!("{}/customers/{}", self.base_url, customer_id);

        let response = self
            .http_client
            .put(&url)
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
            .json::<BrailsCustomerEnvelope>()
            .await
            .map_err(|e| e.to_string())?;

        if !body.status {
            return Err(body
                .message
                .unwrap_or_else(|| "Brails returned failure".to_string()));
        }

        Ok(body.data)
    }

    /// Lists customers
    pub async fn list_customers(&self, api_key: &str) -> Result<BrailsCustomerList, String> {
        let url = format!("{}/customers", self.base_url);

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
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
            .json::<BrailsCustomerListEnvelope>()
            .await
            .map_err(|e| e.to_string())?;

        if !body.status {
            return Err(body
                .message
                .unwrap_or_else(|| "Brails returned failure".to_string()));
        }

        Ok(body.data)
    }

    /// (REFACTORED) Calls Brails POST /virtual-accounts (Source 351)
    pub async fn create_virtual_account(
        &self,
        api_key: &str,
        payload: BrailsVirtualAccountPayload,
    ) -> Result<BrailsVirtualAccountData, String> {
        let url = format!("{}/virtual-accounts", self.base_url);

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
            .json::<BrailsVirtualAccountResponse>()
            .await
            .map_err(|e| e.to_string())?;

        if !body.status {
            return Err(body.data.status); // Pass back the reason
        }

        Ok(body.data)
    }

    /// Calls Brails to register a user for card services (this is the KYC check)
    pub async fn register_card_user(
        &self,
        api_key: &str,
        payload: BrailsRegisterUserPayload,
    ) -> Result<BrailsRegisterUserData, String> {
        let url = format!("{}/virtualcards/registercarduser", self.base_url);

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
            .json::<BrailsRegisterUserResponse>()
            .await
            .map_err(|e| e.to_string())?;

        if !body.status {
            // Pass back the reason, e.g., "PENDING" or "BVN_MISMATCH"
            return Err(body.data.kyc_status);
        }

        Ok(body.data)
    }
}
