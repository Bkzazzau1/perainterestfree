use crate::auth::jwt::Claims; // To get the user ID
use crate::user_service::models::{
    ContactOtpRequest, ContactOtpVerifyRequest, KycStepPayload, OnboardingPayload,
    UpdateDisplayProfile,
}; // Merged imports
use crate::{error::AppError, AppState};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use chrono::Utc;
use serde_json::json;
use tracing::info; // <-- ADD THIS

fn optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|inner| {
        let trimmed = inner.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

async fn ensure_profile_row(state: &AppState, user_id: uuid::Uuid) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        INSERT INTO user_profiles (user_id)
        VALUES ($1)
        ON CONFLICT (user_id) DO NOTHING
        "#,
        user_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(())
}

#[axum::debug_handler]
pub async fn get_kyc_profile(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let profile = sqlx::query!(
        r#"
        SELECT
            country,
            surname,
            first_name,
            middle_name,
            dob,
            address,
            contact_phone,
            contact_email,
            bvn_encrypted IS NOT NULL AS "has_bvn!",
            nin_encrypted IS NOT NULL AS "has_nin!",
            id_type,
            occupation,
            employer,
            income_source,
            annual_income,
            id_doc_path,
            proof_of_address_path,
            bank_stmt_path,
            selfie_path,
            COALESCE(biometric_opt_in, FALSE) AS "biometric_opt_in!",
            id_scan_completed_at IS NOT NULL AS "id_scan_done!",
            face_scan_completed_at IS NOT NULL AS "face_scan_done!",
            locale,
            updated_at AS "updated_at?"
        FROM user_profiles
        WHERE user_id = $1
        "#,
        claims.sub
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    if let Some(profile) = profile {
        return Ok((
            StatusCode::OK,
            Json(json!({
                "submitted": true,
                "country": profile.country,
                "surname": profile.surname,
                "firstName": profile.first_name,
                "middleName": profile.middle_name,
                "dob": profile.dob,
                "address": profile.address,
                "phone": profile.contact_phone,
                "email": profile.contact_email,
                "hasBvn": profile.has_bvn,
                "hasNin": profile.has_nin,
                "idType": profile.id_type,
                "occupation": profile.occupation,
                "employer": profile.employer,
                "incomeSource": profile.income_source,
                "annualIncome": profile.annual_income,
                "idDocPath": profile.id_doc_path,
                "proofOfAddressPath": profile.proof_of_address_path,
                "bankStmtPath": profile.bank_stmt_path,
                "selfiePath": profile.selfie_path,
                "biometricOptIn": profile.biometric_opt_in,
                "idScanDone": profile.id_scan_done,
                "faceScanDone": profile.face_scan_done,
                "locale": profile.locale,
                "updatedAt": profile.updated_at,
            })),
        ));
    }

    Ok((StatusCode::OK, Json(json!({ "submitted": false }))))
}

#[axum::debug_handler]
pub async fn get_kyc_status(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let record = sqlx::query!(
        r#"
        SELECT
            u.kyc_status,
            p.user_id IS NOT NULL AS "submitted!",
            COALESCE(p.biometric_opt_in, FALSE) AS "biometric_opt_in!",
            p.id_scan_completed_at IS NOT NULL AS "id_scan_done!",
            p.face_scan_completed_at IS NOT NULL AS "face_scan_done!",
            p.updated_at AS "updated_at?"
        FROM users u
        LEFT JOIN user_profiles p ON p.user_id = u.id
        WHERE u.id = $1
        "#,
        claims.sub
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "kycStatus": record.kyc_status,
            "submitted": record.submitted,
            "biometricOptIn": record.biometric_opt_in,
            "idScanDone": record.id_scan_done,
            "faceScanDone": record.face_scan_done,
            "updatedAt": record.updated_at,
        })),
    ))
}

#[axum::debug_handler]
pub async fn mark_id_scan_complete(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<KycStepPayload>,
) -> Result<impl IntoResponse, AppError> {
    ensure_profile_row(&state, claims.sub).await?;

    let completed_at = if payload.completed {
        Some(Utc::now())
    } else {
        None
    };
    let path = optional_string(payload.path);

    sqlx::query!(
        r#"
        UPDATE user_profiles
        SET id_scan_completed_at = $1,
            id_doc_path = COALESCE($2, id_doc_path),
            updated_at = NOW()
        WHERE user_id = $3
        "#,
        completed_at,
        path,
        claims.sub
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    info!(user_id = %claims.sub, completed = payload.completed, "Updated KYC ID scan progress");
    Ok((
        StatusCode::OK,
        Json(json!({ "completed": payload.completed })),
    ))
}

#[axum::debug_handler]
pub async fn mark_face_scan_complete(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<KycStepPayload>,
) -> Result<impl IntoResponse, AppError> {
    ensure_profile_row(&state, claims.sub).await?;

    let completed_at = if payload.completed {
        Some(Utc::now())
    } else {
        None
    };
    let path = optional_string(payload.path);

    sqlx::query!(
        r#"
        UPDATE user_profiles
        SET face_scan_completed_at = $1,
            selfie_path = COALESCE($2, selfie_path),
            updated_at = NOW()
        WHERE user_id = $3
        "#,
        completed_at,
        path,
        claims.sub
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    info!(user_id = %claims.sub, completed = payload.completed, "Updated KYC face scan progress");
    Ok((
        StatusCode::OK,
        Json(json!({ "completed": payload.completed })),
    ))
}

/// Handler for PUT /api/v1/user/kyc-profile (REFACTORED)
/// This just saves the KYC data to our local DB.
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn submit_kyc_profile(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<OnboardingPayload>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;
    let phone = optional_string(payload.phone);
    let email = optional_string(payload.email);
    let locale = optional_string(payload.locale);
    let biometric_opt_in = payload.biometric_opt_in;
    let id_scan_completed_at = if payload.id_scan_done.unwrap_or(false) {
        Some(Utc::now())
    } else {
        None
    };
    let face_scan_completed_at = if payload.face_scan_done.unwrap_or(false) {
        Some(Utc::now())
    } else {
        None
    };

    // 1. Encrypt Sensitive Data
    let bvn_encrypted = payload
        .bvn
        .as_deref()
        .map(|bvn| state.crypto_service.encrypt(bvn));

    let nin_encrypted = payload
        .nin
        .as_deref()
        .map(|nin| state.crypto_service.encrypt(nin));

    // 2. Save to 'user_profiles' table (Upsert)
    sqlx::query!(
        r#"
        INSERT INTO user_profiles (
            user_id, country, surname, first_name, middle_name, dob, address,
            bvn_encrypted, nin_encrypted, contact_phone, contact_email,
            id_type, occupation, employer,
            income_source, annual_income, id_doc_path, proof_of_address_path,
            bank_stmt_path, selfie_path, biometric_opt_in,
            id_scan_completed_at, face_scan_completed_at, locale
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
        ON CONFLICT (user_id) DO UPDATE SET
            country = EXCLUDED.country,
            surname = EXCLUDED.surname,
            first_name = EXCLUDED.first_name,
            middle_name = EXCLUDED.middle_name,
            dob = EXCLUDED.dob,
            address = EXCLUDED.address,
            bvn_encrypted = EXCLUDED.bvn_encrypted,
            nin_encrypted = EXCLUDED.nin_encrypted,
            contact_phone = EXCLUDED.contact_phone,
            contact_email = EXCLUDED.contact_email,
            id_type = EXCLUDED.id_type,
            occupation = EXCLUDED.occupation,
            employer = EXCLUDED.employer,
            income_source = EXCLUDED.income_source,
            annual_income = EXCLUDED.annual_income,
            id_doc_path = EXCLUDED.id_doc_path,
            proof_of_address_path = EXCLUDED.proof_of_address_path,
            bank_stmt_path = EXCLUDED.bank_stmt_path,
            selfie_path = EXCLUDED.selfie_path,
            biometric_opt_in = COALESCE(EXCLUDED.biometric_opt_in, user_profiles.biometric_opt_in),
            id_scan_completed_at = COALESCE(EXCLUDED.id_scan_completed_at, user_profiles.id_scan_completed_at),
            face_scan_completed_at = COALESCE(EXCLUDED.face_scan_completed_at, user_profiles.face_scan_completed_at),
            locale = EXCLUDED.locale,
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
        phone,
        email,
        payload.id_type,
        payload.occupation,
        payload.employer,
        payload.income_source,
        payload.annual_income,
        payload.id_doc_path,
        payload.proof_of_address_path,
        payload.bank_stmt_path,
        payload.selfie_path,
        biometric_opt_in,
        id_scan_completed_at,
        face_scan_completed_at,
        locale
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // 3. Set status to 'pending' (internal status)
    sqlx::query!(
        "UPDATE users SET kyc_status = 'pending' WHERE id = $1",
        user_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    tracing::info!(user_id = %user_id, "Saved KYC profile data locally");
    Ok((StatusCode::OK, "Profile data saved."))
}

/// Handler for PUT /api/v1/user/display-profile
#[axum::debug_handler] // <-- CORE FIX APPLIED
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

    // --- ADDED ---
    info!(user_id = %claims.sub, "User updated display profile");
    // -------------

    Ok((StatusCode::OK, "Profile updated"))
}

/// Handler for POST /api/v1/user/contact-otp/request
#[axum::debug_handler]
pub async fn request_contact_otp(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<ContactOtpRequest>,
) -> Result<impl IntoResponse, AppError> {
    let channel = crate::otp_service::normalize_channel(&payload.channel);
    if channel != "EMAIL" && channel != "PHONE" {
        return Err(AppError::ProviderError("INVALID_CHANNEL".to_string()));
    }

    let target = crate::otp_service::normalize_target(&channel, &payload.value);
    if target.is_empty() {
        return Err(AppError::ProviderError("VALUE_REQUIRED".to_string()));
    }

    let purpose = if channel == "EMAIL" {
        "PROFILE_EMAIL"
    } else {
        "PROFILE_PHONE"
    };

    let code = crate::otp_service::create_otp(
        &state.db_pool,
        Some(claims.sub),
        purpose,
        &channel,
        &target,
    )
    .await?;

    if channel == "EMAIL" {
        state
            .email_service
            .send_email(
                target.clone(),
                "Verify your Pera profile".to_string(),
                format!(
                    "Your Pera verification code is {}. It expires in 10 minutes.",
                    code
                ),
            )
            .await;
    } else {
        tracing::debug!(user_id = %claims.sub, target = %target, code = %code, "Generated phone OTP for profile verification");
    }

    Ok((StatusCode::OK, Json(json!({ "sent": true }))))
}

/// Handler for POST /api/v1/user/contact-otp/verify
#[axum::debug_handler]
pub async fn verify_contact_otp(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<ContactOtpVerifyRequest>,
) -> Result<impl IntoResponse, AppError> {
    let channel = crate::otp_service::normalize_channel(&payload.channel);
    if channel != "EMAIL" && channel != "PHONE" {
        return Err(AppError::ProviderError("INVALID_CHANNEL".to_string()));
    }

    let target = crate::otp_service::normalize_target(&channel, &payload.value);
    if target.is_empty() {
        return Err(AppError::ProviderError("VALUE_REQUIRED".to_string()));
    }

    let purpose = if channel == "EMAIL" {
        "PROFILE_EMAIL"
    } else {
        "PROFILE_PHONE"
    };

    crate::otp_service::mark_verified(
        &state.db_pool,
        Some(claims.sub),
        purpose,
        &channel,
        &target,
        &payload.code,
    )
    .await?;

    let result = if channel == "EMAIL" {
        sqlx::query!(
            "UPDATE users SET email = $1, email_verified_at = NOW() WHERE id = $2",
            target.clone(),
            claims.sub
        )
        .execute(&state.db_pool)
        .await
    } else {
        sqlx::query!(
            "UPDATE users SET phone = $1, phone_verified_at = NOW() WHERE id = $2",
            target.clone(),
            claims.sub
        )
        .execute(&state.db_pool)
        .await
    };

    result.map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.is_unique_violation() {
                return AppError::ProviderError("CONTACT_ALREADY_IN_USE".to_string());
            }
        }
        AppError::DatabaseError(e)
    })?;

    crate::otp_service::consume(&state.db_pool, Some(claims.sub), purpose, &channel, &target)
        .await?;

    info!(user_id = %claims.sub, channel = %channel, "Verified contact detail");
    Ok((StatusCode::OK, Json(json!({ "verified": true }))))
}
