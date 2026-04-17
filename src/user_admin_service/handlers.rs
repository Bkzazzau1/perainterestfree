use crate::admin_auth_service::models::AdminClaims;
use crate::notification_service::service as notification_service; // <-- ADDED
use crate::user_admin_service::models::{KycUpdatePayload, UserFullProfile, UserSummary};
use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct UserQuery {
    pub q: Option<String>, // Search
}

/// Handler for GET /api/v1/admin/users
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn list_users(
    State(state): State<AppState>,
    Extension(admin_claims): Extension<AdminClaims>, // Verify admin
    Query(query): Query<UserQuery>,
) -> Result<impl IntoResponse, AppError> {
    let search = query
        .q
        .map_or("".to_string(), |q| format!("%{}%", q.to_lowercase()));

    let users = sqlx::query_as!(
        UserSummary,
        r#"
        SELECT id, display_name, email, phone, kyc_status, created_at
        FROM users
        WHERE ($1 = '' OR LOWER(email) LIKE $1 OR LOWER(phone) LIKE $1)
        ORDER BY created_at DESC
        LIMIT 100
        "#,
        search
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    info!(admin_id = %admin_claims.sub, "Viewed user list");
    Ok((StatusCode::OK, Json(users)))
}

/// Handler for GET /api/v1/admin/users/:id
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn get_user_detail(
    State(state): State<AppState>,
    Extension(admin_claims): Extension<AdminClaims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Get auth info
    let auth_info = sqlx::query!(
        "SELECT id, display_name, email, phone, kyc_status, created_at FROM users WHERE id = $1",
        id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // 2. Get profile info
    let profile_info = sqlx::query!("SELECT * FROM user_profiles WHERE user_id = $1", id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?;

    // 3. Decrypt sensitive fields
    let (bvn, nin, profile) = if let Some(profile) = profile_info {
        let bvn = profile
            .bvn_encrypted
            .as_ref()
            .and_then(|b| state.crypto_service.decrypt(b).ok());
        let nin = profile
            .nin_encrypted
            .as_ref()
            .and_then(|n| state.crypto_service.decrypt(n).ok());
        (bvn, nin, Some(profile))
    } else {
        (None, None, None)
    };

    let full_profile = UserFullProfile {
        id: auth_info.id,
        display_name: auth_info.display_name,
        email: auth_info.email,
        phone: auth_info.phone,
        kyc_status: auth_info.kyc_status,
        created_at: auth_info.created_at,

        country: profile.as_ref().and_then(|p| p.country.clone()),
        surname: profile.as_ref().and_then(|p| p.surname.clone()),
        first_name: profile.as_ref().and_then(|p| p.first_name.clone()),
        dob: profile.as_ref().and_then(|p| p.dob),
        address: profile.as_ref().and_then(|p| p.address.clone()),
        bvn: bvn,
        nin: nin,
        id_type: profile.as_ref().and_then(|p| p.id_type.clone()),
        occupation: profile.as_ref().and_then(|p| p.occupation.clone()),
        middle_name: profile.as_ref().and_then(|p| p.middle_name.clone()),
        employer: profile.as_ref().and_then(|p| p.employer.clone()),
        income_source: profile.as_ref().and_then(|p| p.income_source.clone()),
        annual_income: profile.as_ref().and_then(|p| p.annual_income.clone()),
        id_doc_path: profile.as_ref().and_then(|p| p.id_doc_path.clone()),
        proof_of_address_path: profile
            .as_ref()
            .and_then(|p| p.proof_of_address_path.clone()),
        bank_stmt_path: profile.as_ref().and_then(|p| p.bank_stmt_path.clone()),
        selfie_path: profile.as_ref().and_then(|p| p.selfie_path.clone()),
    };

    info!(admin_id = %admin_claims.sub, user_id = %id, "Viewed user detail");
    Ok((StatusCode::OK, Json(full_profile)))
}

/// Handler for POST /api/v1/admin/users/:id/kyc
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn update_kyc_status(
    State(state): State<AppState>,
    Extension(admin_claims): Extension<AdminClaims>,
    Path(id): Path<Uuid>,
    Json(payload): Json<KycUpdatePayload>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Check for valid status
    let new_status = payload.new_status.to_lowercase();
    if new_status != "verified" && new_status != "unverified" && new_status != "pending" {
        return Err(AppError::ProviderError("Invalid status".to_string()));
    }

    // 2. Update the user's status
    sqlx::query!(
        "UPDATE users SET kyc_status = $1 WHERE id = $2",
        new_status,
        id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // 3. (TODO) Log this action in an admin audit log

    // 4. Send a notification to the user
    let title = format!("KYC Status Updated: {}", new_status);
    let body = format!(
        "Your verification status was updated by an admin. Reason: {}",
        payload.reason
    );
    notification_service::create_notification(&state.db_pool, id, &title, &body).await;

    info!(
        admin_id = %admin_claims.sub,
        user_id = %id,
        new_status = %new_status,
        "Updated user KYC status"
    );

    Ok((StatusCode::OK, "KYC status updated"))
}
