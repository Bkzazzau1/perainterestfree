use serde::{Deserialize, Serialize};

// Matches 'zakat_service.dart'
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZakatRates {
    pub gold_per_gram: f64,
    pub silver_per_gram: f64,
}

// Payload for POST /islamic/pay-zakat
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayZakatPayload {
    // Amount in NGN major units (e.g., 5000.50)
    pub amount: f64,
    pub pin: String,
    // The ID of the charity we are paying
    pub beneficiary_id: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UmrahAgency {
    pub id: String,
    pub code: String,
    pub name: String,
    pub city: String,
    pub country: String,
    pub account_name: String,
    pub description: String,
    pub qr_enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayUmrahPayload {
    pub agency_id: String,
    pub amount: f64,
    pub pin: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveAgencyQuery {
    pub code: String,
}

pub fn approved_umrah_agencies() -> Vec<UmrahAgency> {
    vec![
        UmrahAgency {
            id: "umrah-makkah-guides".to_string(),
            code: "UMR001".to_string(),
            name: "Makkah Guides Travel".to_string(),
            city: "Lagos".to_string(),
            country: "Nigeria".to_string(),
            account_name: "Makkah Guides Travel".to_string(),
            description: "Licensed Umrah agency with QR-ready collections.".to_string(),
            qr_enabled: true,
        },
        UmrahAgency {
            id: "umrah-madina-journeys".to_string(),
            code: "UMR002".to_string(),
            name: "Madina Journeys".to_string(),
            city: "Abuja".to_string(),
            country: "Nigeria".to_string(),
            account_name: "Madina Journeys Limited".to_string(),
            description: "Group Umrah packages and guided travel support.".to_string(),
            qr_enabled: true,
        },
        UmrahAgency {
            id: "umrah-haram-routes".to_string(),
            code: "UMR003".to_string(),
            name: "Haram Routes".to_string(),
            city: "Kano".to_string(),
            country: "Nigeria".to_string(),
            account_name: "Haram Routes Services".to_string(),
            description: "Approved agency for family and premium Umrah trips.".to_string(),
            qr_enabled: true,
        },
        UmrahAgency {
            id: "umrah-safa-marwa".to_string(),
            code: "UMR004".to_string(),
            name: "Safa & Marwa Travel".to_string(),
            city: "Ilorin".to_string(),
            country: "Nigeria".to_string(),
            account_name: "Safa and Marwa Travel".to_string(),
            description: "Weekend consultation and installment-friendly planning.".to_string(),
            qr_enabled: false,
        },
    ]
}

pub fn find_umrah_agency_by_id(id: &str) -> Option<UmrahAgency> {
    approved_umrah_agencies()
        .into_iter()
        .find(|agency| agency.id.eq_ignore_ascii_case(id))
}

pub fn find_umrah_agency_by_code(code: &str) -> Option<UmrahAgency> {
    approved_umrah_agencies().into_iter().find(|agency| {
        agency.code.eq_ignore_ascii_case(code) || agency.id.eq_ignore_ascii_case(code)
    })
}
