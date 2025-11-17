use serde::{Deserialize, Serialize};
use serde_json::Value;

// Matches 'BillProvider' in your controller
#[derive(Serialize, Clone)]
pub struct BillProvider {
    pub code: String,
    pub name: String,
}

// Matches 'products' in your controller
#[derive(Serialize, Clone)]
pub struct BillProduct {
    pub code: String,
    pub name: String,
    pub price: i64, // Price in minor units
}

// Matches 'FormFieldSpec' in your controller
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FormFieldSpec {
    pub key: String,
    pub label: String,
    pub required: bool,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<Value>>,
}

// Payload for the /pay endpoint
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillPaymentPayload {
    pub service: String, // "electricity", "cable", etc.
    pub provider_code: String,
    pub pin: String,
    
    // The amount for services like 'electricity'
    #[serde(default)]
    pub amount_minor: i64, 
    
    // The code for services like 'cable'
    pub product_code: Option<String>,
    
    // All other dynamic fields (smartcard, meter_number, etc.)
    pub fields: Value, 
}