use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use tracing::error; // <-- ADDED

// Define our custom error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Internal server error")]
    InternalServerError,

    #[error("Background task failed")]
    TaskPanic,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Hashing error: {0}")]
    HashError(#[from] bcrypt::BcryptError),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token creation error")]
    TokenCreationError,

    #[error("Authentication required")]
    Unauthorized,

    #[error("Token decoding error: {0}")]
    TokenDecodeError(#[from] jsonwebtoken::errors::Error),

    #[error("KYC profile is incomplete")]
    KycIncomplete,

    #[error("Forbidden")]
    Forbidden,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Account already exists")]
    AccountAlreadyExists,

    #[error("Transaction declined: {0}")]
    TransactionDeclined(String),
}

// Implement 'IntoResponse' for 'AppError'
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // --- UPDATED ---
        // Log the full error with tracing
        // We capture the error message and the full debug info
        error!(error = %self, debug = ?self, "Request processing failed");
        // ---------------

        let (status, error_message) = match self {
            // --- 500 Internal Server Errors ---
            AppError::DatabaseError(_)
            | AppError::InternalServerError
            | AppError::TaskPanic
            | AppError::HashError(_)
            | AppError::TokenCreationError
            | AppError::TokenDecodeError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal server error occurred".to_string(),
            ),

            // --- 401 Unauthorized ---
            AppError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string())
            }
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Authentication required".to_string(),
            ),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string()),

            // --- 400 Bad Request ---
            AppError::ProviderError(msg) => {
                // e.g., "INVALID_BVN" from the provider
                (StatusCode::BAD_REQUEST, msg)
            }
            AppError::TransactionDeclined(reason) => {
                // This tells the payment processor "we are declining this"
                (StatusCode::BAD_REQUEST, reason)
            }

            // --- 409 Conflict ---
            AppError::AccountAlreadyExists => (
                StatusCode::CONFLICT,
                "Virtual account already exists".to_string(),
            ),

            // --- 404 Not Found ---
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),

            // --- 412 Precondition Failed ---
            AppError::KycIncomplete => (
                StatusCode::PRECONDITION_FAILED,
                "KYC profile is incomplete".to_string(),
            ),
        };

        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}
