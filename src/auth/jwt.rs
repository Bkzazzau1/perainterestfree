use crate::error::AppError;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The claims (payload) part of the JWT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid, // User ID
    pub jti: Uuid, // Session ID
    pub iat: i64,
    pub exp: i64,
}

/// Creates a new JWT token for a given user ID and session ID.
pub fn create_token(
    user_id: Uuid,
    session_id: Uuid, // <-- Added session_id
    secret: &str,
) -> Result<String, AppError> {
    let now = Utc::now();
    let expires_in = Duration::days(7);

    let claims = Claims {
        sub: user_id,
        jti: session_id, // <-- Set session_id as jti
        iat: now.timestamp(),
        exp: (now + expires_in).timestamp(),
    };

    let header = Header::default();
    let key = EncodingKey::from_secret(secret.as_bytes());

    encode(&header, &claims, &key).map_err(|_| AppError::TokenCreationError)
}

/// Decodes a JWT token, validates it, and returns the claims.
pub fn decode_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    // We don't need to change validation; it will check 'exp'
    let validation = Validation::default();

    decode::<Claims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(AppError::TokenDecodeError)
}
