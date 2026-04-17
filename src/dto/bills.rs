#![allow(dead_code)]

// ==================================================
// File: src/dto/bills.rs
// Purpose: All Payscribe Bills DTOs (Requests + Responses)
// Scope: Airtime, Data, Electricity, Cable TV, ePins
// ==================================================

use serde::{Deserialize, Serialize};
use serde_json::Value;

/* ─────────────────────────────────────────────
   Generic Payscribe Response Wrapper
   (Used by ALL bill endpoints)
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize)]
pub struct PayscribeResponse<T> {
    pub status: bool,
    pub description: Option<String>,
    pub message: Option<T>,
    pub status_code: Option<i32>,
}

/* ─────────────────────────────────────────────
   Cable TV — Bouquets
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize)]
pub struct BouquetsMessage {
    pub details: Vec<BouquetItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BouquetItem {
    pub id: String,
    pub name: String,
    pub alias: Option<String>,
    pub amount: i64,

    /// Some providers expose extra pricing rules
    #[serde(default, rename = "priceOptions")]
    pub price_options: Vec<Value>,
}

/* ─────────────────────────────────────────────
   ePins — List
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize)]
pub struct EpinsMessage {
    pub details: Vec<EpinGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EpinGroup {
    pub name: String,
    pub collection: Vec<EpinCollection>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EpinCollection {
    pub id: String,
    pub name: String,
    pub amount: i64,
    pub available: i64,
}

/* ─────────────────────────────────────────────
   ePins — Vend (Request)
───────────────────────────────────────────── */

#[derive(Debug, Serialize)]
pub struct VendEpinRequest {
    pub qty: i64,
    pub id: String,

    /// Internal Pera reference (mapped to "ref")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}

impl VendEpinRequest {
    /// Convert struct into Payscribe wire format
    pub fn into_wire(mut self) -> Value {
        let mut v = serde_json::to_value(&self).expect("VendEpinRequest serialization failed");

        if let Some(r) = self.ref_.take() {
            v["ref"] = Value::String(r);
        }

        if let Some(obj) = v.as_object_mut() {
            obj.remove("ref_");
        }

        v
    }
}

/* ─────────────────────────────────────────────
   ePins — Vend (Response)
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize)]
pub struct VendEpinMessage {
    pub details: VendEpinDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VendEpinDetails {
    pub trans_id: String,
    pub ref_: Option<String>,
    pub qty: i64,
    pub amount: i64,
    pub discount: i64,
    pub total_charge: i64,
    pub title: String,
    pub epins: Vec<VendedPin>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VendedPin {
    pub name: String,
    pub pin: String,
    pub serial: String,
}

/* ─────────────────────────────────────────────
   Airtime
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize)]
pub struct AirtimeMessage {
    pub details: AirtimeDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AirtimeDetails {
    pub trans_id: String,
    pub ref_: Option<String>,
    pub network: String,
    pub amount: i64,
    pub phone_number: String,
    pub status: String,
}

/* ─────────────────────────────────────────────
   Data Bundles
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize)]
pub struct DataPlansMessage {
    pub details: Vec<DataPlan>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataPlan {
    pub code: String,
    pub name: String,
    pub amount: i64,
    pub network: String,
}

/* ─────────────────────────────────────────────
   Electricity — Validate
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize)]
pub struct ElectricityValidationMessage {
    pub customer_name: String,
    pub customer_address: Option<String>,

    #[serde(rename = "productCode")]
    pub product_code: String,

    #[serde(rename = "productToken")]
    pub product_token: String,
}

/* ─────────────────────────────────────────────
   Electricity — Vend
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize)]
pub struct ElectricityVendMessage {
    pub details: ElectricityVendDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElectricityVendDetails {
    pub trans_id: String,
    pub ref_: Option<String>,
    pub token: Option<String>,
    pub amount: i64,
    pub units: Option<String>,
    pub status: String,
}

/* ─────────────────────────────────────────────
   Cable TV — Validate
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize)]
pub struct CableValidationMessage {
    pub customer_name: String,

    #[serde(rename = "productCode")]
    pub product_code: String,

    #[serde(rename = "productToken")]
    pub product_token: String,
}

/* ─────────────────────────────────────────────
   Cable TV — Vend
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize)]
pub struct CableVendMessage {
    pub details: CableVendDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CableVendDetails {
    pub trans_id: String,
    pub ref_: Option<String>,
    pub amount: i64,
    pub status: String,
}
