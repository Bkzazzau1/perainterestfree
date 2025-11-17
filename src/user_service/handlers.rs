use axum::{extract::State, http::StatusCode, response::IntoResponse, Json, Extension};
use crate::{error::AppError, AppState};
use crate::auth::jwt::Claims; // To get the user ID
use crate::user_service::models::{OnboardingPayload, UpdateDisplayProfile}; // Merged imports
use uuid::Uuid;

/// Handler for PUT /api/v1/user/kyc-profile
/// Receives the full onboarding/KYC payload.
pub async fn submit_kyc_profile(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>, // Get user ID from token
    Json(payload): Json<OnboardingPayload>,
) -> Result<impl IntoResponse, AppError> {
    
    let user_id = claims.sub;

    // --- 1. Encrypt Sensitive Data ---
    // We only encrypt if the value is provided
    let bvn_encrypted = payload.bvn
        .as_deref()
        .map(|bvn| state.crypto_service.encrypt(bvn));
        
    let nin_encrypted = payload.nin
        .as_deref()
        .map(|nin| state.crypto_service.encrypt(nin));

    // --- 2. Save to 'user_profiles' table ---
    // "ON CONFLICT" makes this an "upsert" (create or update)
    sqlx::query!(
        r#"
        INSERT INTO user_profiles (
            user_id, country, surname, first_name, middle_name, dob, address,
            bvn_encrypted, nin_encrypted, id_type, occupation, employer,
            income_source, annual_income, id_doc_path, proof_of_address_path,
            bank_stmt_path, selfie_path
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
            $14, $15, $16, $17, $18
        )
        ON CONFLICT (user_id) DO UPDATE SET
            country = EXCLUDED.country,
            surname = EXCLUDED.surname,
            first_name = EXCLUDED.first_name,
            middle_name = EXCLUDED.middle_name,
            dob = EXCLUDED.dob,
            address = EXCLUDED.address,
            bvn_encrypted = EXCLUDED.bvn_encrypted,
            nin_encrypted = EXCLUDED.nin_encrypted,
            id_type = EXCLUDED.id_type,
            occupation = EXCLUDED.occupation,
            employer = EXCLUDED.employer,
            income_source = EXCLUDED.income_source,
            annual_income = EXCLUDED.annual_income,
            id_doc_path = EXCLUDED.id_doc_path,
            proof_of_address_path = EXCLUDED.proof_of_address_path,
            bank_stmt_path = EXCLUDED.bank_stmt_path,
            selfie_path = EXCLUDED.selfie_path,
            updated_at = NOW()
        "#,
        user_id,
        payload.country,
        payload.surname,
        payload.first_name,
        payload.middle_name,
        payload.dob,
        payload.address,
        bvn_encrypted,
        nin_encrypted,
        payload.id_type,
        payload.occupation,
        payload.employer,
        payload.income_source,
        payload.annual_income,
        payload.id_doc_path,
        payload.proof_of_address_path,
        payload.bank_stmt_path,
        payload.selfie_path
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- 3. Update kyc_status to 'pending' ---
    // This flags the user's auth status for review
    sqlx::query!(
        "UPDATE users SET kyc_status = 'pending' WHERE id = $1",
        user_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- 4. TODO: MVP Verification Logic ---
    // This is where you would:
    // 1. Check if (bvn_encrypted.is_some())
    // 2. Make the (mocked) call to "Brails"
    // 3. Get back the name, e.g., "John Doe"
    // 4. Compare "John Doe" to "payload.first_name + payload.surname"
    // 5. If match, you could *immediately* set kyc_status = 'verified'

    println!("✅ Profile updated and KYC set to 'pending' for user: {}", user_id);
    
    Ok((StatusCode::OK, "Profile submitted successfully."))
}

/// Handler for PUT /api/v1/user/display-profile
pub async fn update_display_profile(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<UpdateDisplayProfile>,
) -> Result<impl IntoResponse, AppError> {
    
    sqlx::query!(
        "UPDATE users SET display_name = $1 WHERE id = $2",
        payload.display_name,
        claims.sub
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok((StatusCode::OK, "Profile updated"))
}