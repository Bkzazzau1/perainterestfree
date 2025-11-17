use serde::{Deserialize, Serialize};
use uuid::Uuid; // <-- ADD THIS

// ... RegisterUser and LoginUser structs ...
#[derive(Deserialize)]
pub struct RegisterUser {
    pub email: String,
    pub otp: String,
    pub password: String,
    pub phone: String,
}
#[derive(Deserialize)]
pub struct LoginUser {
    pub id: String,
    pub password: String,
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
    pub email: String,
    pub phone: String,
    pub kyc_status: String,
}
// -------------------------