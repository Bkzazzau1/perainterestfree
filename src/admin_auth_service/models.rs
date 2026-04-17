use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Separate claims for Admin JWTs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminClaims {
    pub sub: Uuid,    // Admin User ID
    pub role: String, // "super_admin", "support"
    pub iat: i64,
    pub exp: i64,
}

#[derive(Deserialize)]
pub struct AdminLoginPayload {
    pub email: String,
    pub password: String,
}
