use axum::{http::StatusCode, response::IntoResponse, Json};
use crate::app_data_service::models::AppConfigResponse;
use crate::error::AppError; // <-- Added import

/// Handler for GET /api/v1/app/config
pub async fn get_app_config() -> Result<impl IntoResponse, AppError> {
    // This data is hardcoded but could be read from a config file
    let config = AppConfigResponse {
        referral_code: "PERA1Services Limited".to_string(), // Corrected from referral_view.dart
        daily_limit_fmt: "₦10,000".to_string(),
        monthly_limit_fmt: "₦30,000".to_string(),
        app_version: "1.0.0".to_string(),
        build_number: "1".to_string(),
        certificate_url: "https://pera.com/certificates/islamic_governance.pdf".to_string(),
    };
    
    Ok((StatusCode::OK, Json(config)))
}