use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::QueryBuilder;
use uuid::Uuid;

use crate::auth::jwt::Claims;
use crate::cash_withdrawal_service::models::{
    CashWithdrawalConfig, CashWithdrawalRow, CreateWithdrawalRequest, PartnerActionResponse,
    PartnerConfirmRequest, PartnerListQuery, PartnerReadyRequest, PartnerReadyResponse,
    PickupInstructions, QuoteRequest, QuoteResponse, WithdrawalDetailResponse,
    WithdrawalHistoryItem,
};
use crate::cash_withdrawal_service::service::{
    amount_to_minor, compute_quote, create_hold_and_transaction, ensure_ng_and_kyc,
    ensure_partner_role, ensure_pin, generate_pickup_code, generate_reference, log_audit,
    normalize_uppercase, pickup_expiry, refund_hold_and_wallet, should_rate_limit,
    validate_currency, validate_method, SUPPORTED_CURRENCIES, SUPPORTED_METHODS,
};
use crate::{error::AppError, AppState};

/// GET /api/v1/cash-withdrawal/config
pub async fn get_config(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let city_rows = sqlx::query!(
        r#"
        SELECT DISTINCT city
        FROM partner_locations
        WHERE is_active = TRUE
        ORDER BY city
        "#
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    let delivery_row = sqlx::query!(
        r#"SELECT COUNT(*)::BIGINT AS count FROM partner_locations WHERE is_active = TRUE AND supports_delivery = TRUE"#
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    let cities = city_rows.into_iter().map(|r| r.city).collect();
    let config = CashWithdrawalConfig {
        supported_currencies: SUPPORTED_CURRENCIES.iter().map(|s| s.to_string()).collect(),
        methods: SUPPORTED_METHODS.iter().map(|s| s.to_string()).collect(),
        supports_delivery: delivery_row.count.unwrap_or(0) > 0,
        pickup_expiry_hours: crate::cash_withdrawal_service::service::DEFAULT_PICKUP_EXPIRY_HOURS,
        cities,
    };

    Ok((StatusCode::OK, Json(config)))
}

/// POST /api/v1/cash-withdrawal/quote
pub async fn get_quote(Json(payload): Json<QuoteRequest>) -> Result<impl IntoResponse, AppError> {
    let currency = normalize_uppercase(&payload.currency);
    let method = normalize_uppercase(&payload.method);
    validate_currency(&currency)?;
    validate_method(&method)?;

    let amount_minor = amount_to_minor(payload.amount)?;
    let (fee_minor, total_debit_minor) = compute_quote(amount_minor);

    let response = QuoteResponse {
        fee_minor,
        total_debit_minor,
        expiry_hours: crate::cash_withdrawal_service::service::DEFAULT_PICKUP_EXPIRY_HOURS,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// POST /api/v1/cash-withdrawal
pub async fn create_withdrawal(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateWithdrawalRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;
    let currency = normalize_uppercase(&payload.currency);
    let method = normalize_uppercase(&payload.method);

    validate_currency(&currency)?;
    validate_method(&method)?;

    ensure_ng_and_kyc(&state.db_pool, user_id).await?;
    ensure_pin(&state.db_pool, user_id, payload.pin).await?;

    let amount_minor = amount_to_minor(payload.amount)?;
    let (fee_minor, total_debit_minor) = compute_quote(amount_minor);
    let reference = generate_reference();

    let delivery_address = payload.delivery_address.or_else(|| {
        if method == "DELIVERY" {
            payload.location_detail.as_ref().map(|location_detail| {
                json!({
                    "city": payload.city,
                    "address": location_detail
                })
            })
        } else {
            None
        }
    });

    if method == "DELIVERY" && delivery_address.is_none() {
        return Err(AppError::ProviderError(
            "DELIVERY_ADDRESS_REQUIRED".to_string(),
        ));
    }

    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    let (_, debit_tx_id, hold_id) =
        create_hold_and_transaction(&mut tx, user_id, &currency, total_debit_minor, &reference)
            .await?;

    let row = sqlx::query_as::<_, CashWithdrawalRow>(
        r#"
        INSERT INTO cash_withdrawals (
            reference, user_id, partner_org_id, location_id, currency, method, amount_minor,
            fee_minor, total_debit_minor, requested_city, location_detail, status,
            pickup_code_hash, pickup_code_expires_at, failed_attempts, last_failed_at,
            delivery_address, debit_transaction_id, hold_id
        )
        VALUES ($1, $2, NULL, $3, $4, $5, $6, $7, $8, $9, $10, 'PENDING',
                NULL, NULL, 0, NULL, $11, $12, $13)
        RETURNING
            id, reference, user_id, partner_org_id, location_id, currency, method,
            amount_minor, fee_minor, total_debit_minor, requested_city, location_detail,
            status, pickup_code_expires_at, delivery_address, failed_attempts,
            created_at, updated_at
        "#,
    )
    .bind(reference)
    .bind(user_id)
    .bind(payload.location_id)
    .bind(currency)
    .bind(method)
    .bind(amount_minor)
    .bind(fee_minor)
    .bind(total_debit_minor)
    .bind(payload.city)
    .bind(payload.location_detail)
    .bind(delivery_address)
    .bind(debit_tx_id)
    .bind(hold_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    tx.commit().await.map_err(AppError::DatabaseError)?;

    log_audit(
        &state.db_pool,
        Some(user_id),
        "cash_withdrawal_created",
        "cash_withdrawal",
        row.id,
        json!({"reference": row.reference, "amount_minor": row.amount_minor, "currency": row.currency}),
    ).await?;

    let response = WithdrawalDetailResponse {
        reference: row.reference,
        currency: row.currency,
        method: row.method,
        amount_minor: row.amount_minor,
        fee_minor: row.fee_minor,
        total_debit_minor: row.total_debit_minor,
        city: row.requested_city,
        location_detail: row.location_detail,
        status: row.status,
        pickup_instructions: None,
        delivery_address: row.delivery_address,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/cash-withdrawal/{reference}
pub async fn get_withdrawal(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(reference): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;
    let record = sqlx::query_as::<_, CashWithdrawalRow>(
        r#"
        SELECT
            id,
            reference,
            user_id,
            partner_org_id,
            location_id,
            currency,
            method,
            amount_minor,
            fee_minor,
            total_debit_minor,
            requested_city,
            location_detail,
            status,
            pickup_code_expires_at,
            delivery_address,
            failed_attempts,
            created_at,
            updated_at
        FROM cash_withdrawals
        WHERE reference = $1 AND user_id = $2
        "#,
    )
    .bind(reference)
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::NotFound("cash_withdrawal".to_string()))?;

    let pickup_instructions = if record.status == "READY" {
        Some(PickupInstructions {
            location_id: record.location_id,
            city: record.requested_city.clone(),
            address: record.location_detail.clone(),
            expires_at: record.pickup_code_expires_at,
        })
    } else {
        None
    };

    let delivery_address = if record.status == "READY" || record.status == "COLLECTED" {
        record.delivery_address
    } else {
        None
    };

    let response = WithdrawalDetailResponse {
        reference: record.reference,
        currency: record.currency,
        method: record.method,
        amount_minor: record.amount_minor,
        fee_minor: record.fee_minor,
        total_debit_minor: record.total_debit_minor,
        city: record.requested_city,
        location_detail: record.location_detail,
        status: record.status,
        pickup_instructions,
        delivery_address,
        created_at: record.created_at,
        updated_at: record.updated_at,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// GET /api/v1/cash-withdrawal/history
pub async fn get_history(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;
    let items = sqlx::query_as::<_, WithdrawalHistoryItem>(
        r#"
        SELECT
            reference,
            currency,
            method,
            amount_minor,
            fee_minor,
            total_debit_minor,
            requested_city AS city,
            status,
            created_at
        FROM cash_withdrawals
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok((StatusCode::OK, Json(items)))
}

/// GET /api/v1/partner/bdc/withdrawals
pub async fn partner_list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<PartnerListQuery>,
) -> Result<impl IntoResponse, AppError> {
    let ctx = ensure_partner_role(&state.db_pool, claims.sub, "BDC_PARTNER").await?;

    let mut builder = QueryBuilder::new(
        r#"
        SELECT
            cw.reference,
            cw.currency,
            cw.method,
            cw.amount_minor,
            cw.fee_minor,
            cw.total_debit_minor,
            cw.requested_city AS city,
            cw.status,
            cw.created_at
        FROM cash_withdrawals cw
        LEFT JOIN partner_locations pl ON cw.location_id = pl.id
        WHERE (cw.partner_org_id = "#,
    );

    builder.push_bind(ctx.partner_org_id);
    builder.push(")");

    if let Some(status) = query.status {
        builder.push(" AND cw.status = ");
        builder.push_bind(status);
    }
    if let Some(city) = query.city {
        builder.push(" AND (pl.city = ");
        builder.push_bind(city);
        builder.push(")");
    }
    if let Some(currency) = query.currency {
        builder.push(" AND cw.currency = ");
        builder.push_bind(currency);
    }
    if let Some(start) = query.start_date {
        builder.push(" AND cw.created_at >= ");
        builder.push_bind(start);
    }
    if let Some(end) = query.end_date {
        builder.push(" AND cw.created_at <= ");
        builder.push_bind(end);
    }

    builder.push(" ORDER BY cw.created_at DESC LIMIT 100");

    let query = builder.build_query_as::<WithdrawalHistoryItem>();

    let rows = query
        .fetch_all(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok((StatusCode::OK, Json(rows)))
}

/// POST /api/v1/partner/bdc/withdrawals/{reference}/ready
pub async fn partner_ready(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(reference): Path<String>,
    Json(payload): Json<PartnerReadyRequest>,
) -> Result<impl IntoResponse, AppError> {
    let ctx = ensure_partner_role(&state.db_pool, claims.sub, "BDC_PARTNER").await?;

    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;
    let record = sqlx::query!(
        r#"
        SELECT id, user_id, status, partner_org_id, method, location_id
        FROM cash_withdrawals
        WHERE reference = $1
        FOR UPDATE
        "#,
        reference
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::NotFound("cash_withdrawal".to_string()))?;

    if record.status != "PENDING" {
        return Err(AppError::ProviderError("INVALID_STATUS".to_string()));
    }

    if let Some(org_id) = record.partner_org_id {
        if org_id != ctx.partner_org_id {
            return Err(AppError::Forbidden);
        }
    }

    // Validate location if provided
    let mut location_id = record.location_id;
    if let Some(loc_id) = payload.location_id {
        let loc = sqlx::query!(
            r#"
            SELECT id, supports_pickup, supports_delivery
            FROM partner_locations
            WHERE id = $1 AND partner_org_id = $2 AND is_active = TRUE
            "#,
            loc_id,
            ctx.partner_org_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ProviderError("LOCATION_NOT_FOUND".to_string()))?;

        if record.method == "PICKUP" && !loc.supports_pickup {
            return Err(AppError::ProviderError(
                "LOCATION_NOT_PICKUP_READY".to_string(),
            ));
        }
        if record.method == "DELIVERY" && !loc.supports_delivery {
            return Err(AppError::ProviderError(
                "LOCATION_NOT_DELIVERY_READY".to_string(),
            ));
        }
        location_id = Some(loc.id);
    }

    let (pickup_code, pickup_hash) = generate_pickup_code().await?;
    let expires_at = pickup_expiry(payload.expiry_hours);

    sqlx::query!(
        r#"
        UPDATE cash_withdrawals
        SET status = 'READY',
            partner_org_id = $2,
            location_id = COALESCE($3, location_id),
            pickup_code_hash = $4,
            pickup_code_expires_at = $5,
            updated_at = NOW()
        WHERE id = $1
        "#,
        record.id,
        ctx.partner_org_id,
        location_id,
        pickup_hash,
        expires_at
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    tx.commit().await.map_err(AppError::DatabaseError)?;

    log_audit(
        &state.db_pool,
        Some(ctx.user_id),
        "marked_ready",
        "cash_withdrawal",
        record.id,
        json!({"reference": reference, "partner_org_id": ctx.partner_org_id}),
    )
    .await?;

    let resp = PartnerReadyResponse {
        reference,
        pickup_code,
        expires_at,
    };

    Ok((StatusCode::OK, Json(resp)))
}

/// POST /api/v1/partner/bdc/withdrawals/{reference}/confirm
pub async fn partner_confirm(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(reference): Path<String>,
    Json(payload): Json<PartnerConfirmRequest>,
) -> Result<impl IntoResponse, AppError> {
    let ctx = ensure_partner_role(&state.db_pool, claims.sub, "BDC_PARTNER").await?;

    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;
    let record = sqlx::query!(
        r#"
        SELECT id, user_id, status, partner_org_id, method, pickup_code_hash,
               pickup_code_expires_at, failed_attempts, last_failed_at,
               hold_id, debit_transaction_id
        FROM cash_withdrawals
        WHERE reference = $1
        FOR UPDATE
        "#,
        reference
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::NotFound("cash_withdrawal".to_string()))?;

    if record.status != "READY" {
        return Err(AppError::ProviderError("INVALID_STATUS".to_string()));
    }

    if let Some(org_id) = record.partner_org_id {
        if org_id != ctx.partner_org_id {
            return Err(AppError::Forbidden);
        }
    }

    let now = Utc::now();
    if let Some(expiry) = record.pickup_code_expires_at {
        if expiry < now {
            sqlx::query!(
                r#"UPDATE cash_withdrawals SET status = 'EXPIRED', updated_at = NOW() WHERE id = $1"#,
                record.id
            )
            .execute(&mut *tx)
            .await
            .map_err(AppError::DatabaseError)?;

            refund_hold_and_wallet(&mut tx, &reference, record.hold_id, record.user_id).await?;
            tx.commit().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::ProviderError("PICKUP_CODE_EXPIRED".to_string()));
        }
    }

    if should_rate_limit(record.failed_attempts, record.last_failed_at) {
        return Err(AppError::ProviderError("TOO_MANY_ATTEMPTS".to_string()));
    }

    if record.method == "PICKUP" {
        let code = payload
            .pickup_code
            .clone()
            .ok_or(AppError::ProviderError("PICKUP_CODE_REQUIRED".to_string()))?;

        let hash = record
            .pickup_code_hash
            .clone()
            .ok_or(AppError::ProviderError("PICKUP_CODE_MISSING".to_string()))?;

        let valid = crate::auth::security::verify_value(code, hash).await?;
        if !valid {
            sqlx::query!(
                r#"
                UPDATE cash_withdrawals
                SET failed_attempts = failed_attempts + 1, last_failed_at = NOW()
                WHERE id = $1
                "#,
                record.id
            )
            .execute(&mut *tx)
            .await
            .map_err(AppError::DatabaseError)?;
            tx.commit().await.ok();
            return Err(AppError::InvalidCredentials);
        }
    } else if payload.delivered.unwrap_or(false) == false {
        return Err(AppError::ProviderError(
            "DELIVERY_CONFIRMATION_REQUIRED".to_string(),
        ));
    }

    sqlx::query!(
        r#"
        UPDATE cash_withdrawals
        SET status = 'COLLECTED',
            partner_org_id = COALESCE($2, partner_org_id),
            failed_attempts = 0,
            updated_at = NOW()
        WHERE id = $1
        "#,
        record.id,
        record.partner_org_id.or(Some(ctx.partner_org_id))
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    sqlx::query!(
        r#"UPDATE wallet_holds SET status = 'CONSUMED', updated_at = NOW(), released_at = NOW() WHERE id = $1"#,
        record.hold_id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    sqlx::query!(
        r#"UPDATE transactions SET status = 'completed' WHERE id = $1"#,
        record.debit_transaction_id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    tx.commit().await.map_err(AppError::DatabaseError)?;

    log_audit(
        &state.db_pool,
        Some(ctx.user_id),
        "collected_confirmed",
        "cash_withdrawal",
        record.id,
        json!({"reference": reference, "partner_org_id": ctx.partner_org_id, "payload": payload.proof}),
    ).await?;

    let resp = PartnerActionResponse {
        reference,
        status: "COLLECTED".to_string(),
    };

    Ok((StatusCode::OK, Json(resp)))
}

/// Travel partner stubs (role-checked by ensure_partner_role)
pub async fn travel_list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    ensure_partner_role(&state.db_pool, claims.sub, "TRAVEL_AGENT").await?;
    Ok((StatusCode::OK, Json(Vec::<Value>::new())))
}

pub async fn travel_confirm(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    ensure_partner_role(&state.db_pool, claims.sub, "TRAVEL_AGENT").await?;
    let resp = PartnerActionResponse {
        reference: id.to_string(),
        status: "CONFIRMED".to_string(),
    };
    Ok((StatusCode::OK, Json(resp)))
}

pub async fn travel_delivered(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    ensure_partner_role(&state.db_pool, claims.sub, "TRAVEL_AGENT").await?;
    let resp = PartnerActionResponse {
        reference: id.to_string(),
        status: "DELIVERED".to_string(),
    };
    Ok((StatusCode::OK, Json(resp)))
}
