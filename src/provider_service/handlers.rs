use axum::{extract::State, http::StatusCode, response::IntoResponse, Json, Extension};
use crate::{error::AppError, AppState};
use crate::auth::jwt::Claims;
use serde::Serialize;
use uuid::Uuid;

// Struct to fetch the KYC data we need
struct KycData {
    surname: String,
    first_name: String,
    bvn_encrypted: String,
}

// The successful response
#[derive(Serialize)]
struct AccountResponse {
    bank_name: String,
    account_number: String,
    account_name: String,
}

/// Handler for POST /api/v1/accounts/create
/// Creates a new NGN virtual account via Brails
pub async fn create_virtual_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {

    let user_id = claims.sub;

    // --- 1. Fetch User's KYC Profile ---
    // We only fetch the *exact* data we need.
    let profile = sqlx::query_as!(
        KycData,
        r#"
        SELECT surname, first_name, bvn_encrypted
        FROM user_profiles
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::KycIncomplete)?; // User has no profile

    // --- 2. Validate we have the required data ---
    if profile.surname.is_empty() || profile.first_name.is_empty() {
        return Err(AppError::KycIncomplete);
    }
    let bvn_encrypted = profile.bvn_encrypted.ok_or(AppError::KycIncomplete)?;

    // --- 3. Decrypt BVN (Security) ---
    let bvn = state.crypto_service.decrypt(&bvn_encrypted)
        .map_err(|_| AppError::ProviderError("BVN decryption failed".to_string()))?;

    // --- 4. Call Brails Client ---
    // This is a blocking (CPU-bound) call, so we use spawn_blocking
    let brails_client = state.brails_client.clone();
    let brails_result = tokio::task::spawn_blocking(move || {
        brails_client.create_virtual_account(&profile.first_name, &profile.surname, &bvn)
    })
    .await
    .map_err(|_| AppError::TaskPanic)?; // Task failed
    
    let brails_account = brails_result
        .map_err(|e| AppError::ProviderError(e))?; // Brails returned an error

    // --- 5. MVP LOGIC: Compare Names ---
    // Brails gives us the official name, e.g., "DOE JOHN"
    // We compare it to the name on the profile, "John Doe"
    let profile_name = format!("{} {}", profile.surname, profile.first_name).to_uppercase();
    if brails_account.account_name != profile_name {
        // This is your key check: The user's uploaded name doesn't match their BVN name.
        return Err(AppError::ProviderError("BVN_NAME_MISMATCH".to_string()));
    }

    // --- 6. Save to DB ---
    let account = sqlx::query_as!(
        AccountResponse,
        r#"
        INSERT INTO virtual_accounts (user_id, bank_name, account_number, account_name)
        VALUES ($1, $2, $3, $4)
        RETURNING bank_name, account_number, account_name
        "#,
        user_id,
        brails_account.bank_name,
        brails_account.account_number,
        brails_account.account_name
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        // Handle "account already exists"
        if let Some(db_err) = e.as_database_error() {
            if db_err.is_unique_violation() {
                return AppError::AccountAlreadyExists;
            }
        }
        AppError::DatabaseError(e)
    })?;

    // --- 7. VERIFY USER'S KYC ---
    // This is the final step. We now trust this user.
    sqlx::query!(
        "UPDATE users SET kyc_status = 'verified' WHERE id = $1",
        user_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;
    
    println!("✅ Virtual account created AND KYC verified for user: {}", user_id);

    Ok((StatusCode::CREATED, Json(account)))
}