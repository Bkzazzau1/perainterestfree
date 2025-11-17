use axum::{
    extract::{ConnectInfo, Query, State}, // <-- Added ConnectInfo
    http::StatusCode,
    response::IntoResponse,
    Json, Extension,
};
use axum::headers::UserAgent; // <-- Added
use axum::TypedHeader; // <-- Added
use std::net::SocketAddr; // <-- Added
use crate::{error::AppError, AppState};
use crate::auth::{jwt::Claims, security::verify_value};
use crate::bills_service::{
    mock_data,
    models::{BillPaymentPayload, BillProduct},
};
use serde::Deserialize;
use serde_json::json;
use tracing::info;

// --- Schema Handlers ---

#[derive(Deserialize)]
pub struct ServiceQuery {
    service: String,
    #[serde(default)]
    provider: String,
}

#[derive(Deserialize)]
pub struct ProductQuery {
    provider: String,
}

pub async fn get_providers(
    Query(query): Query<ServiceQuery>,
) -> Result<impl IntoResponse, AppError> {
    let providers = mock_data::get_mock_providers(&query.service);
    Ok((StatusCode::OK, Json(providers)))
}

pub async fn get_products(
    Query(query): Query<ProductQuery>,
) -> Result<impl IntoResponse, AppError> {
    let products = mock_data::get_mock_products(&query.provider);
    Ok((StatusCode::OK, Json(products)))
}

pub async fn get_schema(
    Query(query): Query<ServiceQuery>,
) -> Result<impl IntoResponse, AppError> {
    let schema = mock_data::get_mock_schema(&query.service, &query.provider);
    Ok((StatusCode::OK, Json(schema)))
}


// --- Payment Handler ---

pub async fn pay_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ConnectInfo(ip): ConnectInfo<SocketAddr>, // <-- Added IP
    TypedHeader(user_agent): TypedHeader<UserAgent>, // <-- Added User-Agent
    Json(payload): Json<BillPaymentPayload>,
) -> Result<impl IntoResponse, AppError> {
    
    let user_id = claims.sub;

    // --- 1. Security: Verify PIN ---
    let user_pin_hash = sqlx::query!("SELECT pin_hash FROM users WHERE id = $1", user_id)
        .fetch_optional(&state.db_pool)
        .await.map_err(AppError::DatabaseError)?
        .and_then(|u| u.pin_hash)
        .ok_or(AppError::ProviderError("PIN_NOT_SET".to_string()))?;

    if !verify_value(payload.pin, user_pin_hash).await? {
        return Err(AppError::InvalidCredentials);
    }

    // --- 2. Determine Amount ---
    let amount_to_debit = if payload.amount_minor > 0 {
        payload.amount_minor
    } else if let Some(product_code) = &payload.product_code {
        let products = mock_data::get_mock_products(&payload.provider_code);
        let product = products.iter().find(|p| p.code == *product_code)
            .ok_or(AppError::ProviderError("Invalid product".to_string()))?;
        product.price
    } else {
        return Err(AppError::ProviderError("Invalid amount".to_string()));
    };

    if amount_to_debit <= 0 {
        return Err(AppError::ProviderError("Amount must be positive".to_string()));
    }

    // --- 3. Start ATOMIC Transaction ---
    let mut tx = state.db_pool.begin().await.map_err(AppError::DatabaseError)?;

    // --- 4. Get NGN Wallet and Lock Row ---
    let wallet = sqlx::query!(
        r#"
        SELECT id, balance_minor
        FROM wallets
        WHERE user_id = $1 AND currency = 'NGN'
        FOR UPDATE
        "#,
        user_id
    )
    .fetch_optional(&mut *tx)
    .await.map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError("NGN wallet not found".to_string()))?;

    // --- 5. Check Balance ---
    if wallet.balance_minor < amount_to_debit {
        tx.rollback().await.ok();
        return Err(AppError::ProviderError("Insufficient NGN funds".to_string()));
    }

    // --- 6. Debit Wallet ---
    sqlx::query!(
        "UPDATE wallets SET balance_minor = balance_minor - $1 WHERE id = $2",
        amount_to_debit,
        wallet.id
    )
    .execute(&mut *tx)
    .await.map_err(AppError::DatabaseError)?;

    // --- 7. Create Transaction Record (UPDATED) ---
    let reference = payload.fields
        .get("meter_number")
        .or_else(|| payload.fields.get("smartcard"))
        .or_else(|| payload.fields.get("customer_ref"))
        .and_then(|v| v.as_str())
        .unwrap_or("N/A")
        .to_string();
    
    let ip_address = ip.to_string();
    let user_agent_str = user_agent.to_string();
    let metadata = json!({
        "service": payload.service,
        "product": payload.product_code,
        "fields": payload.fields
    });

    sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title,
            counterparty, reference, metadata, ip_address, user_agent
        )
        VALUES ($1, $2, 'bill_payment', 'completed', $3, 'NGN', $4, $5, $6, $7, $8, $9)
        "#,
        user_id,
        wallet.id,
        -amount_to_debit,
        format!("{} Bill", payload.provider_code), // title
        payload.provider_code, // counterparty
        reference, // reference
        metadata, // metadata
        ip_address, // <-- Save IP
        user_agent_str // <-- Save User-Agent
    )
    .execute(&mut *tx)
    .await.map_err(AppError::DatabaseError)?;
    
    // --- 8. (MOCK) Call Bills Provider ---
    
    // --- 9. Commit Transaction ---
    tx.commit().await.map_err(AppError::DatabaseError)?;

    // --- 10. Log ---
    info!(
        provider = %payload.provider_code,
        service = %payload.service,
        amount = amount_to_debit,
        user_id = %user_id,
        "Processed bill payment"
    );

    Ok((StatusCode::OK, "Payment successful"))
}