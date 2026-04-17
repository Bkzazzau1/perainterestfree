use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde_json::json;
use sqlx::FromRow;
use sqlx::QueryBuilder;
use uuid::Uuid;

use crate::auth::jwt::Claims;
use crate::cash_deposit_service::models::{
    CashDepositConfig, CashDepositRow, CreateDepositRequest, DepositDetailResponse,
    DepositHistoryItem, PartnerDepositAcceptRequest, PartnerDepositActionResponse,
    PartnerDepositRejectRequest,
};
use crate::cash_withdrawal_service::service::{
    amount_to_minor, ensure_ng_and_kyc, ensure_partner_role, log_audit, normalize_uppercase,
};
use crate::error::AppError;
use crate::AppState;

const SUPPORTED_CURRENCIES: [&str; 2] = ["USD", "GBP"];
const MEETING_METHODS: [&str; 2] = ["SAFE_PUBLIC_PLACE", "PICKUP_AT_SELLER_LOCATION"];

#[derive(FromRow)]
struct CityRow {
    city: String,
}

pub async fn get_config(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let cities = sqlx::query_as::<_, CityRow>(
        r#"
        SELECT DISTINCT city
        FROM partner_locations
        WHERE is_active = TRUE
        ORDER BY city
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?
    .into_iter()
    .map(|row| row.city)
    .collect();

    let response = CashDepositConfig {
        supported_currencies: SUPPORTED_CURRENCIES
            .iter()
            .map(|value| value.to_string())
            .collect(),
        meeting_methods: MEETING_METHODS
            .iter()
            .map(|value| value.to_string())
            .collect(),
        cities,
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn create_deposit(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateDepositRequest>,
) -> Result<impl IntoResponse, AppError> {
    let currency = normalize_uppercase(&payload.currency);
    if !SUPPORTED_CURRENCIES.contains(&currency.as_str()) {
        return Err(AppError::ProviderError("UNSUPPORTED_CURRENCY".to_string()));
    }

    let method = normalize_uppercase(&payload.method);
    if !MEETING_METHODS.contains(&method.as_str()) {
        return Err(AppError::ProviderError(
            "UNSUPPORTED_MEETING_METHOD".to_string(),
        ));
    }

    let city = payload.city.trim().to_string();
    if city.is_empty() {
        return Err(AppError::ProviderError("CITY_REQUIRED".to_string()));
    }

    let location_detail = payload.location_detail.trim().to_string();
    if location_detail.is_empty() {
        return Err(AppError::ProviderError(
            "LOCATION_DETAIL_REQUIRED".to_string(),
        ));
    }

    if !payload.safety_confirmed {
        return Err(AppError::ProviderError(
            "SAFETY_CONFIRMATION_REQUIRED".to_string(),
        ));
    }

    ensure_ng_and_kyc(&state.db_pool, claims.sub).await?;

    let amount_minor = amount_to_minor(payload.amount)?;
    let reference = Uuid::new_v4().to_string();

    let row = sqlx::query_as::<_, CashDepositRow>(
        r#"
        INSERT INTO cash_deposits (
            reference, user_id, partner_org_id, location_id, currency, amount_minor,
            requested_city, meeting_method, location_detail, preferred_window,
            safety_confirmed, status, instructions, rejection_reason,
            credited_transaction_id
        )
        VALUES ($1, $2, NULL, NULL, $3, $4, $5, $6, $7, $8, $9,
                'PENDING_SELLER', NULL, NULL, NULL)
        RETURNING
            id, reference, user_id, partner_org_id, location_id, currency,
            amount_minor, requested_city, meeting_method, location_detail,
            preferred_window, safety_confirmed, status, instructions,
            rejection_reason, credited_transaction_id, created_at, updated_at
        "#,
    )
    .bind(reference)
    .bind(claims.sub)
    .bind(currency)
    .bind(amount_minor)
    .bind(city)
    .bind(method)
    .bind(location_detail)
    .bind(payload.preferred_window)
    .bind(payload.safety_confirmed)
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    log_audit(
        &state.db_pool,
        Some(claims.sub),
        "cash_deposit_created",
        "cash_deposit",
        row.id,
        json!({
            "reference": row.reference,
            "amount_minor": row.amount_minor,
            "currency": row.currency,
            "city": row.requested_city
        }),
    )
    .await?;

    let response = DepositDetailResponse {
        reference: row.reference,
        currency: row.currency,
        amount_minor: row.amount_minor,
        city: row.requested_city,
        method: row.meeting_method,
        location_detail: row.location_detail,
        preferred_window: row.preferred_window,
        safety_confirmed: row.safety_confirmed,
        status: row.status,
        instructions: row.instructions,
        rejection_reason: row.rejection_reason,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_deposit(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(reference): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query_as::<_, CashDepositRow>(
        r#"
        SELECT
            id, reference, user_id, partner_org_id, location_id, currency,
            amount_minor, requested_city, meeting_method, location_detail,
            preferred_window, safety_confirmed, status, instructions,
            rejection_reason, credited_transaction_id, created_at, updated_at
        FROM cash_deposits
        WHERE reference = $1 AND user_id = $2
        "#,
    )
    .bind(reference)
    .bind(claims.sub)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::NotFound("cash_deposit".to_string()))?;

    let response = DepositDetailResponse {
        reference: row.reference,
        currency: row.currency,
        amount_minor: row.amount_minor,
        city: row.requested_city,
        method: row.meeting_method,
        location_detail: row.location_detail,
        preferred_window: row.preferred_window,
        safety_confirmed: row.safety_confirmed,
        status: row.status,
        instructions: row.instructions,
        rejection_reason: row.rejection_reason,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_history(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let items = sqlx::query_as::<_, DepositHistoryItem>(
        r#"
        SELECT
            reference,
            currency,
            amount_minor,
            requested_city AS city,
            meeting_method AS method,
            status,
            created_at
        FROM cash_deposits
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(claims.sub)
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok((StatusCode::OK, Json(items)))
}

pub async fn partner_list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let ctx = ensure_partner_role(&state.db_pool, claims.sub, "BDC_PARTNER").await?;

    let mut builder = QueryBuilder::new(
        r#"
        SELECT
            reference,
            currency,
            amount_minor,
            requested_city AS city,
            meeting_method AS method,
            status,
            created_at
        FROM cash_deposits
        WHERE (partner_org_id = "#,
    );
    builder.push_bind(ctx.partner_org_id);
    builder.push(" OR partner_org_id IS NULL) ORDER BY created_at DESC LIMIT 100");

    let rows = builder
        .build_query_as::<DepositHistoryItem>()
        .fetch_all(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok((StatusCode::OK, Json(rows)))
}

pub async fn partner_accept(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(reference): Path<String>,
    Json(payload): Json<PartnerDepositAcceptRequest>,
) -> Result<impl IntoResponse, AppError> {
    let ctx = ensure_partner_role(&state.db_pool, claims.sub, "BDC_PARTNER").await?;
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    let record = sqlx::query!(
        r#"
        SELECT id, status, partner_org_id
        FROM cash_deposits
        WHERE reference = $1
        FOR UPDATE
        "#,
        reference
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::NotFound("cash_deposit".to_string()))?;

    if record.status != "PENDING_SELLER" {
        return Err(AppError::ProviderError("INVALID_STATUS".to_string()));
    }

    if let Some(org_id) = record.partner_org_id {
        if org_id != ctx.partner_org_id {
            return Err(AppError::Forbidden);
        }
    }

    if let Some(location_id) = payload.location_id {
        sqlx::query!(
            r#"
            SELECT id
            FROM partner_locations
            WHERE id = $1 AND partner_org_id = $2 AND is_active = TRUE
            "#,
            location_id,
            ctx.partner_org_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ProviderError("LOCATION_NOT_FOUND".to_string()))?;
    }

    sqlx::query!(
        r#"
        UPDATE cash_deposits
        SET status = 'ACCEPTED',
            partner_org_id = $2,
            location_id = COALESCE($3, location_id),
            instructions = $4,
            updated_at = NOW()
        WHERE id = $1
        "#,
        record.id,
        ctx.partner_org_id,
        payload.location_id,
        payload.instructions
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    tx.commit().await.map_err(AppError::DatabaseError)?;

    log_audit(
        &state.db_pool,
        Some(ctx.user_id),
        "cash_deposit_accepted",
        "cash_deposit",
        record.id,
        json!({ "reference": reference, "partner_org_id": ctx.partner_org_id }),
    )
    .await?;

    Ok((
        StatusCode::OK,
        Json(PartnerDepositActionResponse {
            reference,
            status: "ACCEPTED".to_string(),
        }),
    ))
}

pub async fn partner_complete(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(reference): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let ctx = ensure_partner_role(&state.db_pool, claims.sub, "BDC_PARTNER").await?;
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    let record = sqlx::query!(
        r#"
        SELECT id, user_id, partner_org_id, currency, amount_minor, status
        FROM cash_deposits
        WHERE reference = $1
        FOR UPDATE
        "#,
        reference
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::NotFound("cash_deposit".to_string()))?;

    if record.status != "ACCEPTED" {
        return Err(AppError::ProviderError("INVALID_STATUS".to_string()));
    }

    if let Some(org_id) = record.partner_org_id {
        if org_id != ctx.partner_org_id {
            return Err(AppError::Forbidden);
        }
    } else {
        return Err(AppError::Forbidden);
    }

    let wallet = sqlx::query!(
        r#"
        SELECT id
        FROM wallets
        WHERE user_id = $1 AND currency = $2
        FOR UPDATE
        "#,
        record.user_id,
        record.currency
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError("WALLET_NOT_FOUND".to_string()))?;

    sqlx::query!(
        r#"
        UPDATE wallets
        SET balance_minor = balance_minor + $1, updated_at = NOW()
        WHERE id = $2
        "#,
        record.amount_minor,
        wallet.id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    let transaction = sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title, reference, metadata
        )
        VALUES ($1, $2, 'cash_deposit', 'completed', $3, $4, $5, $6, $7)
        RETURNING id
        "#,
        record.user_id,
        wallet.id,
        record.amount_minor,
        record.currency,
        format!("Cash deposit {}", record.currency),
        reference,
        json!({ "reference": reference, "partner_org_id": ctx.partner_org_id })
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    sqlx::query!(
        r#"
        UPDATE cash_deposits
        SET status = 'COMPLETED',
            credited_transaction_id = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
        record.id,
        transaction.id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    tx.commit().await.map_err(AppError::DatabaseError)?;

    log_audit(
        &state.db_pool,
        Some(ctx.user_id),
        "cash_deposit_completed",
        "cash_deposit",
        record.id,
        json!({ "reference": reference, "partner_org_id": ctx.partner_org_id }),
    )
    .await?;

    Ok((
        StatusCode::OK,
        Json(PartnerDepositActionResponse {
            reference,
            status: "COMPLETED".to_string(),
        }),
    ))
}

pub async fn partner_reject(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(reference): Path<String>,
    Json(payload): Json<PartnerDepositRejectRequest>,
) -> Result<impl IntoResponse, AppError> {
    let ctx = ensure_partner_role(&state.db_pool, claims.sub, "BDC_PARTNER").await?;

    let record = sqlx::query!(
        r#"
        SELECT id, status, partner_org_id
        FROM cash_deposits
        WHERE reference = $1
        FOR UPDATE
        "#,
        reference
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::NotFound("cash_deposit".to_string()))?;

    if record.status != "PENDING_SELLER" && record.status != "ACCEPTED" {
        return Err(AppError::ProviderError("INVALID_STATUS".to_string()));
    }

    if let Some(org_id) = record.partner_org_id {
        if org_id != ctx.partner_org_id {
            return Err(AppError::Forbidden);
        }
    }

    sqlx::query!(
        r#"
        UPDATE cash_deposits
        SET status = 'REJECTED',
            partner_org_id = COALESCE(partner_org_id, $2),
            rejection_reason = $3,
            updated_at = NOW()
        WHERE id = $1
        "#,
        record.id,
        ctx.partner_org_id,
        payload.reason
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    log_audit(
        &state.db_pool,
        Some(ctx.user_id),
        "cash_deposit_rejected",
        "cash_deposit",
        record.id,
        json!({ "reference": reference, "partner_org_id": ctx.partner_org_id }),
    )
    .await?;

    Ok((
        StatusCode::OK,
        Json(PartnerDepositActionResponse {
            reference,
            status: "REJECTED".to_string(),
        }),
    ))
}
