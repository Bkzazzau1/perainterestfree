use crate::app_data_service::models::AppConfigResponse;
use crate::error::AppError; // <-- Added import
use axum::{http::StatusCode, response::IntoResponse, Json};
use tracing::info; // <-- ADD THIS

/// Handler for GET /api/v1/app/config
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn get_app_config() -> Result<impl IntoResponse, AppError> {
    // This data is hardcoded but could be read from a config file
    let config = AppConfigResponse {
        referral_code: "PERA1234".to_string(),
        daily_limit_fmt: "₦10,000".to_string(),
        monthly_limit_fmt: "₦30,000".to_string(),
        app_version: "1.0.0".to_string(),
        build_number: "1".to_string(),
        company_name: "Pera Fide Services Limited".to_string(),
        copyright_text: "© 2026 Pera Fide Services Limited".to_string(),
        support_email: "support@pera.com".to_string(),
        certificate_url: "https://pera.com/certificates/islamic_governance.pdf".to_string(),
    };

    // --- ADDED ---
    info!("Fetched app config");
    // -------------

    Ok((StatusCode::OK, Json(config)))
}
