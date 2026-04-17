use crate::auth::jwt::Claims;
use crate::{error::AppError, AppState};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde::Serialize;
// --- ADD THESE ---
use crate::brails_client::BrailsVirtualAccountPayload;
use chrono::NaiveDate;
use tracing::{debug, info}; // <-- UPDATED
                            // -----------------

// The successful response
#[derive(Serialize)]
struct AccountResponse {
    bank_name: String,
    account_number: String,
    account_name: String,
}

// Helper struct for our local data
struct UserData {
    email: String,
    phone: String,
    first_name: String,
    surname: String,
    bvn: String,
    dob: NaiveDate,
}

/// Handler for POST /api/v1/accounts/create (REFACTORED)
/// Creates a new NGN virtual account via Brails
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn create_virtual_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;

    // --- 1. Fetch User's Data ---
    // We need data from both 'users' and 'user_profiles'
    let user_data = sqlx::query!(
        r#"
        SELECT
            u.email,
            u.phone,
            p.first_name,
            p.surname,
            p.bvn_encrypted,
            p.dob
        FROM users u
        JOIN user_profiles p ON u.id = p.user_id
        WHERE u.id = $1
        "#,
        user_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::KycIncomplete)?; // User must have a profile

    // --- 2. Validate and Decrypt Data ---
    let bvn = user_data
        .bvn_encrypted
        .and_then(|b| state.crypto_service.decrypt(&b).ok())
        .ok_or(AppError::ProviderError("BVN is required".to_string()))?;

    let user = UserData {
        email: user_data.email,
        phone: user_data.phone,
        first_name: user_data.first_name.ok_or(AppError::KycIncomplete)?,
        surname: user_data.surname.ok_or(AppError::KycIncomplete)?,
        dob: user_data.dob.ok_or(AppError::KycIncomplete)?,
        bvn,
    };

    // --- 3. Get Brails API key ---
    let settings = crate::admin_settings_service::service::get_all_settings(&state.db_pool).await?;
    let api_key = settings
        .get("brails_api_key")
        .ok_or(AppError::ProviderError(
            "Brails API key not set".to_string(),
        ))?;

    // --- 4. Build Brails Payload (Source 351) ---
    let brails_payload = BrailsVirtualAccountPayload {
        first_name: user.first_name,
        last_name: user.surname,
        bvn: user.bvn,
        date_of_birth: user.dob.format("%Y-%m-%d").to_string(),
        customer_email: user.email,
        reference: user_id.to_string(), // Use our user_id as the unique reference
        bank: "providus".to_string(),   // (Mock)
        phone_number: user.phone,
    };

    // --- 5. Call Brails Client ---
    let brails_account = state
        .brails_client
        .create_virtual_account(api_key, brails_payload)
        .await
        .map_err(AppError::ProviderError)?;

    // --- 6. Save to DB ---
    let account = sqlx::query_as!(
        AccountResponse,
        r#"
        INSERT INTO virtual_accounts (user_id, bank_name, account_number, account_name)
        VALUES ($1, $2, $3, $4)
        RETURNING bank_name, account_number, account_name
        "#,
        user_id,
        brails_account.bank,
        brails_account.account_number,
        brails_account.account_name
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.is_unique_violation() {
                // --- ADDED ---
                debug!(user_id = %user_id, "Failed to create virtual account: already exists");
                // -------------
                return AppError::AccountAlreadyExists;
            }
        }
        AppError::DatabaseError(e)
    })?;

    info!(user_id = %user_id, "NGN virtual account created");

    Ok((StatusCode::CREATED, Json(account)))
}
