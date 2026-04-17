use serde::{Deserialize, Serialize};
use uuid::Uuid; // <-- ADD THIS

// ... RegisterUser and LoginUser structs ...
#[derive(Deserialize)]
pub struct RegisterUser {
    pub email: String,
    #[allow(dead_code)]
    pub otp: String,
    pub password: String,
    pub phone: String,
}
#[derive(Deserialize)]
pub struct LoginUser {
    pub id: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct OtpRequest {
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Deserialize)]
pub struct VerifyOtpRequest {
    pub code: String,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordResetConfirmRequest {
    pub email: String,
    pub code: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub token: String,
}

// --- ADD THIS STRUCT ---
/// The user data we send in response to /api/v1/me
#[derive(Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub display_name: Option<String>,
    pub email: String,
    pub phone: String,
    pub kyc_status: String,
    pub email_verified: bool,
    pub phone_verified: bool,
}
// -------------------------
