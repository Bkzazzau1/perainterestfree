use serde::Deserialize;
use chrono::NaiveDate; // For 'dob'

#[derive(Deserialize, Debug)]
pub struct OnboardingPayload {
    pub country: Option<String>,
    pub surname: Option<String>,
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    #[serde(default)] // Handle 'dob: null'
    pub dob: Option<NaiveDate>, // Assumes "YYYY-MM-DD" format from client
    pub address: Option<String>,
    
    // We receive sensitive data in plain text from the client (over HTTPS)
    pub bvn: Option<String>,
    pub nin: Option<String>,
    
    pub id_type: Option<String>,
    pub occupation: Option<String>,
    pub employer: Option<String>,
    pub income_source: Option<String>,
    pub annual_income: Option<String>,
    
    // File paths are just strings
    pub id_doc_path: Option<String>,
    pub proof_of_address_path: Option<String>,
    pub bank_stmt_path: Option<String>,
    pub selfie_path: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDisplayProfile {
    pub display_name: String,
}