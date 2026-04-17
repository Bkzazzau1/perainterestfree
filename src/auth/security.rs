use crate::error::AppError;
use tokio::task;

/// Hashes a password or PIN on a blocking thread.
pub async fn hash_value(value: String) -> Result<String, AppError> {
    task::spawn_blocking(move || bcrypt::hash(value, bcrypt::DEFAULT_COST))
        .await
        .map_err(|_| AppError::TaskPanic)? // Handle task panic
        .map_err(AppError::HashError) // Handle hashing error
}

/// Verifies a password or PIN against a hash on a blocking thread.
pub async fn verify_value(value: String, hash: String) -> Result<bool, AppError> {
    task::spawn_blocking(move || bcrypt::verify(value, &hash))
        .await
        .map_err(|_| AppError::TaskPanic)? // Handle task panic
        .map_err(AppError::HashError) // Handle bcrypt error
}
