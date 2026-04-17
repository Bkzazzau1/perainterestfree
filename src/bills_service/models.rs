// FILE: src/bills_service/models.rs

use serde::{Deserialize, Serialize};
use serde_json::Value;

/* ─────────────────────────────────────────────
   Services list
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BillService {
    /// internal/service code (e.g. mtn, glo, ikedc_prepaid, dstv)
    pub code: String,

    /// human readable provider name (e.g. MTN, IKEDC, DSTV)
    pub provider: String,

    /// label for UI (e.g. "MTN Airtime", "IKEDC Prepaid")
    pub label: String,

    /// category/service type (airtime, data, electricity, cable_tv, epin)
    #[serde(rename = "service_type")]
    pub service_type: String,

    /// amounts are in MINOR units (kobo) because your wallet is minor
    pub min_amount: i64,
    pub max_amount: i64,
}

/* ─────────────────────────────────────────────
   Validate Customer
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidatePayload {
    /// "electricity" | "cable_tv"
    pub category: String,

    /// e.g. "ikedc_prepaid" or "dstv"
    pub service_code: String,

    /// meter number / smartcard / decoder number
    pub account_number: String,

    /// required for electricity: "prepaid" | "postpaid" (or provider-specific)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_type: Option<String>,

    /// amount in minor units (kobo) - needed by some providers for validation
    pub amount: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidateResponse {
    pub valid: bool,
    pub customer_name: String,
    pub customer_address: Option<String>,

    /// returned by provider on validation (must be sent to vend)
    pub product_code: String,
    pub product_token: String,

    /// raw provider JSON for debugging/audit
    pub raw_response: Value,
}

/* ─────────────────────────────────────────────
   Pay Bill (Airtime/Data/Electricity/Cable/ePin)
───────────────────────────────────────────── */

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BillPayPayload {
    /// "airtime" | "data" | "electricity" | "cable_tv" | "epin"
    pub category: String,

    /// provider code: mtn/glo/ikedc_prepaid/dstv...
    pub service_code: String,

    /// amount in MINOR units (kobo)
    pub amount: i64,

    /// target account (meter/smartcard/etc). For airtime/data you can reuse as phone if you want,
    /// but we also expose dedicated phone fields below for clarity.
    pub account_number: String,

    /// from validation step (needed for electricity/cable vend)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_token: Option<String>,

    /// required for electricity validation (not required for vend if already validated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_type: Option<String>,

    /// for airtime/data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<String>,

    /// airtime_to_wallet requires "from"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_phone: Option<String>,

    /// user PIN (plain) to be verified against stored hash
    pub pin: String,

    /// our internal reference
    pub reference: String,

    /// provider-specific payload (used by epin vend and future expansion)
    /// Example epin payload could include: { "qty": 1, "id": "...", "ref": "...", "phone": "...", "account": "..." }
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BillPayResponse {
    pub status: String, // "processing"
    pub reference: String,
    pub provider_reference: String,
    pub message: String,
    pub amount: i64,
    pub category: String,
    pub service_code: String,
}

/* ─────────────────────────────────────────────
   Optional: Bills category enum helper (not required)
───────────────────────────────────────────── */

#[allow(dead_code)]
pub mod categories {
    pub const AIRTIME: &str = "airtime";
    pub const DATA: &str = "data";
    pub const ELECTRICITY: &str = "electricity";
    pub const CABLE_TV: &str = "cable_tv";
    pub const EPIN: &str = "epin";
}
