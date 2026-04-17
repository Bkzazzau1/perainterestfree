use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// A summary of a user for the main list
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSummary {
    pub id: Uuid,
    pub display_name: Option<String>,
    pub email: String,
    pub phone: String,
    pub kyc_status: String,
    pub created_at: DateTime<Utc>,
}

// A full user profile for the detail view
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFullProfile {
    // Auth info
    pub id: Uuid,
    pub display_name: Option<String>,
    pub email: String,
    pub phone: String,
    pub kyc_status: String,
    pub created_at: DateTime<Utc>,

    // Profile info (decrypted)
    pub country: Option<String>,
    pub surname: Option<String>,
    pub first_name: Option<String>,
    pub dob: Option<NaiveDate>,
    pub address: Option<String>,
    pub bvn: Option<String>, // Decrypted
    pub nin: Option<String>, // Decrypted
    pub id_type: Option<String>,
    pub occupation: Option<String>,

    // Optional extended profile fields
    pub middle_name: Option<String>,
    pub employer: Option<String>,
    pub income_source: Option<String>,
    pub annual_income: Option<String>,
    pub id_doc_path: Option<String>,
    pub proof_of_address_path: Option<String>,
    pub bank_stmt_path: Option<String>,
    pub selfie_path: Option<String>,
}

// Payload for updating KYC status
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KycUpdatePayload {
    pub new_status: String, // "verified", "unverified"
    pub reason: String,
}
