// FILE: src/bills_service/handlers.rs

use axum::{
    extract::{ConnectInfo, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use axum_extra::headers::UserAgent;
use axum_extra::TypedHeader;
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::auth::{jwt::Claims, security::verify_value};
use crate::bills_service::models::{
    BillPayPayload, BillPayResponse, BillService, ValidatePayload, ValidateResponse,
};
use crate::fraud_service::service as fraud_service;
use crate::{error::AppError, AppState};
use tracing::{debug, info};

#[derive(Deserialize)]
pub struct ServiceQuery {
    pub category: String,
}

#[derive(Deserialize)]
pub struct CableBouquetQuery {
    pub service: String,
}

/// Helper: get a required string field from Option<String>
// FILE: src/bills_service/handlers.rs (top helpers)

fn req_str<'a>(v: &'a Option<String>, err: &str) -> Result<&'a str, AppError> {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::ProviderError(err.to_string()))
}

/// Handler for GET /api/v1/bills/services?category=...
#[axum::debug_handler]
pub async fn get_services(
    Query(query): Query<ServiceQuery>,
) -> Result<impl IntoResponse, AppError> {
    let services: Vec<BillService> = match query.category.as_str() {
        "electricity" => vec![
            BillService {
                code: "ikedc_prepaid".to_string(),
                provider: "IKEDC".to_string(),
                label: "IKEDC Prepaid".to_string(),
                service_type: "prepaid".to_string(),
                min_amount: 10_000,
                max_amount: 50_000_000,
            },
            BillService {
                code: "ikedc_postpaid".to_string(),
                provider: "IKEDC".to_string(),
                label: "IKEDC Postpaid".to_string(),
                service_type: "postpaid".to_string(),
                min_amount: 10_000,
                max_amount: 50_000_000,
            },
        ],
        "airtime" => vec![
            BillService {
                code: "mtn".to_string(),
                provider: "MTN".to_string(),
                label: "MTN Airtime".to_string(),
                service_type: "airtime".to_string(),
                min_amount: 100,
                max_amount: 50_000_000,
            },
            BillService {
                code: "glo".to_string(),
                provider: "GLO".to_string(),
                label: "GLO Airtime".to_string(),
                service_type: "airtime".to_string(),
                min_amount: 100,
                max_amount: 50_000_000,
            },
        ],
        "data" => vec![BillService {
            code: "mtn".to_string(),
            provider: "MTN".to_string(),
            label: "MTN Data".to_string(),
            service_type: "data".to_string(),
            min_amount: 100,
            max_amount: 100_000_000,
        }],
        "cable_tv" => vec![
            BillService {
                code: "dstv".to_string(),
                provider: "DSTV".to_string(),
                label: "DSTV".to_string(),
                service_type: "cable_tv".to_string(),
                min_amount: 100,
                max_amount: 50_000_000,
            },
            BillService {
                code: "gotv".to_string(),
                provider: "GOTV".to_string(),
                label: "GOTV".to_string(),
                service_type: "cable_tv".to_string(),
                min_amount: 100,
                max_amount: 50_000_000,
            },
        ],
        "epin" => vec![BillService {
            code: "epin".to_string(),
            provider: "Payscribe".to_string(),
            label: "ePins".to_string(),
            service_type: "epin".to_string(),
            min_amount: 0,
            max_amount: 0,
        }],
        _ => vec![],
    };

    debug!(category = %query.category, count = services.len(), "Fetched bill services");
    Ok((
        StatusCode::OK,
        Json(json!({ "category": query.category, "services": services })),
    ))
}

/// Handler for POST /api/v1/bills/validate
#[axum::debug_handler]
pub async fn validate_customer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<ValidatePayload>,
) -> Result<impl IntoResponse, AppError> {
    let (validation_data, raw_response) = match payload.category.as_str() {
        "electricity" => {
            let meter_type = payload.meter_type.as_deref().ok_or_else(|| {
                AppError::ProviderError("meter_type is required for electricity".to_string())
            })?;

            let v = state
                .payscribe_client
                .validate_electricity(
                    &payload.account_number,
                    meter_type,
                    &payload.service_code,
                    payload.amount,
                )
                .await
                .map_err(AppError::ProviderError)?;

            (v, json!({"type": "electricity"}))
        }
        "cable_tv" => {
            let v = state
                .payscribe_client
                .validate_cable_tv(&payload.service_code, &payload.account_number)
                .await
                .map_err(AppError::ProviderError)?;

            (v, json!({"type": "cable_tv"}))
        }
        _ => {
            return Err(AppError::ProviderError(
                "Category not supported for validation".to_string(),
            ))
        }
    };

    let response = ValidateResponse {
        valid: true,
        customer_name: validation_data.customer_name,
        customer_address: validation_data.customer_address,
        product_code: validation_data.product_code,
        product_token: validation_data.product_token,
        raw_response,
    };

    info!(
        user_id = %claims.sub,
        category = %payload.category,
        service_code = %payload.service_code,
        customer_name = %response.customer_name,
        "Validated bill customer"
    );

    Ok((StatusCode::OK, Json(response)))
}

/// Handler for POST /api/v1/bills/pay
#[axum::debug_handler]
pub async fn pay_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(ip): ConnectInfo<SocketAddr>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    Json(mut payload): Json<BillPayPayload>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = claims.sub;
    let ip_address = ip.to_string();
    let user_agent_str = user_agent.to_string();

    // Reference: support either String or Option<String> depending on your models.
    // If your BillPayPayload.reference is String => this compiles and works.
    // If it's Option<String> => keep the block below and change reference field type in models accordingly.
    if payload.reference.trim().is_empty() {
        payload.reference = Uuid::new_v4().to_string();
    }

    // 1) Verify PIN
    let pin_hash = sqlx::query!("SELECT pin_hash FROM users WHERE id = $1", user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?
        .and_then(|u| u.pin_hash)
        .ok_or(AppError::ProviderError("PIN_NOT_SET".to_string()))?;

    if !verify_value(payload.pin.clone(), pin_hash).await? {
        debug!(user_id = %user_id, "Bill payment failed: incorrect PIN");
        return Err(AppError::InvalidCredentials);
    }

    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    // Fraud requires &str for "counterparty" or "product"
    // For airtime/data, use service_code
    // For electricity/cable, prefer product_code if provided, else service_code
    let fraud_counterparty: &str = payload
        .product_code
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(payload.service_code.as_str());

    // 2) Fraud check
    fraud_service::check_payment_risk(
        &mut tx,
        user_id,
        payload.amount,
        &payload.category,
        fraud_counterparty,
        "NG",
        &ip_address,
        &user_agent_str,
    )
    .await?;

    // 3) Lock wallet + balance check
    let wallet = sqlx::query!(
        "SELECT id, balance_minor FROM wallets WHERE user_id = $1 AND currency = 'NGN' FOR UPDATE",
        user_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError("NGN wallet not found".to_string()))?;

    if wallet.balance_minor < payload.amount {
        tx.rollback().await.ok();
        debug!(user_id = %user_id, "Bill payment failed: insufficient NGN funds");
        return Err(AppError::ProviderError(
            "Insufficient NGN funds".to_string(),
        ));
    }

    // 4) Debit wallet (MVP: debit before provider call)
    sqlx::query!(
        "UPDATE wallets SET balance_minor = balance_minor - $1 WHERE id = $2",
        payload.amount,
        wallet.id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // 5) Provider call
    let provider_result: Result<(String, Value), AppError> = match payload.category.as_str() {
        "airtime" => {
            let phone = req_str(&payload.phone_number, "phone_number required for airtime")?;
            let from = req_str(&payload.from_phone, "from_phone required for airtime")?;

            let res = state
                .payscribe_client
                .vend_airtime(
                    &payload.service_code,
                    payload.amount,
                    phone,
                    from,
                    &payload.reference,
                )
                .await
                .map_err(AppError::ProviderError)?;

            Ok((
                res.provider_reference.clone(),
                json!({
                    "status": res.status,
                    "provider_reference": res.provider_reference
                }),
            ))
        }

        "data" => {
            let phone = req_str(&payload.phone_number, "phone_number required for data")?;
            let plan_code = payload.service_code.as_str(); // or payload.product_code if your API uses plan code differently

            let res = state
                .payscribe_client
                .vend_data(plan_code, phone, &payload.reference)
                .await
                .map_err(AppError::ProviderError)?;

            Ok((
                res.provider_reference.clone(),
                json!({
                    "status": res.status,
                    "provider_reference": res.provider_reference
                }),
            ))
        }

        "electricity" => {
            let product_code = req_str(
                &payload.product_code,
                "product_code required for electricity vend",
            )?;
            let product_token = req_str(
                &payload.product_token,
                "product_token required for electricity vend",
            )?;

            let res = state
                .payscribe_client
                .vend_electricity(product_code, product_token, &payload.reference)
                .await
                .map_err(AppError::ProviderError)?;

            Ok((
                res.provider_reference.clone(),
                json!({
                    "status": res.status,
                    "provider_reference": res.provider_reference
                }),
            ))
        }

        "cable_tv" => {
            let product_code = req_str(
                &payload.product_code,
                "product_code required for cable_tv vend",
            )?;
            let product_token = req_str(
                &payload.product_token,
                "product_token required for cable_tv vend",
            )?;

            let res = state
                .payscribe_client
                .vend_cable_tv(product_code, product_token, &payload.reference)
                .await
                .map_err(AppError::ProviderError)?;

            Ok((
                res.provider_reference.clone(),
                json!({
                    "status": res.status,
                    "provider_reference": res.provider_reference
                }),
            ))
        }

        "epin" => {
            let raw = payload.extra.clone().ok_or_else(|| {
                AppError::ProviderError("extra payload required for epin vend".to_string())
            })?;

            let res = state
                .payscribe_client
                .vend_epin(&raw)
                .await
                .map_err(AppError::ProviderError)?;

            let provider_ref = res
                .pointer("/message/details/trans_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            Ok((provider_ref, res))
        }

        _ => Err(AppError::ProviderError(
            "Category not supported".to_string(),
        )),
    };

    // Provider fail => refund (MVP)
    let (provider_reference, provider_raw) = match provider_result {
        Ok(v) => v,
        Err(e) => {
            sqlx::query!(
                "UPDATE wallets SET balance_minor = balance_minor + $1 WHERE id = $2",
                payload.amount,
                wallet.id
            )
            .execute(&mut *tx)
            .await
            .map_err(AppError::DatabaseError)?;

            tx.commit().await.map_err(AppError::DatabaseError)?;
            return Err(e);
        }
    };

    // 6) Save transaction
    let meta = json!({
        "category": payload.category,
        "service_code": payload.service_code,
        "product_code": payload.product_code,
        "account_number": payload.account_number,
        "provider_reference": provider_reference,
        "provider_raw": provider_raw,
    });

    sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title,
            counterparty, reference, metadata, ip_address, user_agent
        )
        VALUES ($1, $2, 'bill_payment', $3, $4, 'NGN', $5, $6, $7, $8, $9, $10)
        "#,
        user_id,
        wallet.id,
        "pending",
        -payload.amount,
        format!("Bill Payment: {}", payload.service_code),
        payload.service_code,
        payload.reference,
        meta,
        ip_address,
        user_agent_str
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    tx.commit().await.map_err(AppError::DatabaseError)?;

    info!(
        user_id = %user_id,
        category = %payload.category,
        reference = %payload.reference,
        provider_reference = %provider_reference,
        "Bill payment initiated"
    );

    let response = BillPayResponse {
        status: "processing".to_string(),
        reference: payload.reference,
        provider_reference,
        message: "Bill payment initiated successfully".to_string(),
        amount: payload.amount,
        category: payload.category,
        service_code: payload.service_code,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Handler for GET /api/v1/bills/cable/bouquets?service=dstv
#[axum::debug_handler]
pub async fn get_cable_bouquets(
    State(state): State<AppState>,
    Query(query): Query<CableBouquetQuery>,
) -> Result<impl IntoResponse, AppError> {
    let bouquets = state
        .payscribe_client
        .get_cable_bouquets(&query.service)
        .await
        .map_err(AppError::ProviderError)?;

    debug!(service = %query.service, "Fetched cable bouquets from Payscribe");
    Ok((StatusCode::OK, Json(bouquets)))
}

/// Handler for GET /api/v1/bills/epins
#[axum::debug_handler]
pub async fn get_epins(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let epins = state
        .payscribe_client
        .get_epins()
        .await
        .map_err(AppError::ProviderError)?;

    // return it so it's not "unused"
    Ok((StatusCode::OK, Json(epins)))
}

/// Handler for POST /api/v1/bills/epins/vend
#[axum::debug_handler]
pub async fn vend_epin(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, AppError> {
    let vend_response = state
        .payscribe_client
        .vend_epin(&payload)
        .await
        .map_err(AppError::ProviderError)?;

    info!(user_id = %claims.sub, "Vend epin initiated");
    Ok((StatusCode::CREATED, Json(vend_response)))
}
