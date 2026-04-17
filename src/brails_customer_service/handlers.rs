use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use tracing::info;

use crate::{
    admin_settings_service::service::get_all_settings,
    auth::jwt::Claims,
    brails_client::{
        BrailsCreateCustomerPayload, BrailsCustomer, BrailsCustomerList,
        BrailsUpdateCustomerPayload,
    },
    brails_customer_service::models::{CreateCustomerRequest, UpdateCustomerRequest},
    error::AppError,
    AppState,
};

fn missing_api_key() -> AppError {
    AppError::ProviderError("Brails API key not set".to_string())
}

async fn get_api_key(state: &AppState) -> Result<String, AppError> {
    let settings = get_all_settings(&state.db_pool).await?;
    settings
        .get("brails_api_key")
        .cloned()
        .ok_or_else(missing_api_key)
}

/// POST /api/v1/brails/customers
pub async fn create_customer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateCustomerRequest>,
) -> Result<impl IntoResponse, AppError> {
    let api_key = get_api_key(&state).await?;
    let brails_payload = BrailsCreateCustomerPayload {
        first_name: payload.first_name,
        last_name: payload.last_name,
        email: payload.email,
        phone: payload.phone,
        country_code: payload.country_code,
    };

    let customer: BrailsCustomer = state
        .brails_client
        .create_customer(&api_key, brails_payload)
        .await
        .map_err(AppError::ProviderError)?;

    info!(user_id = %claims.sub, "Created Brails customer");

    Ok((StatusCode::CREATED, Json(customer)))
}

/// GET /api/v1/brails/customers/{id}
pub async fn get_customer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(customer_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let api_key = get_api_key(&state).await?;

    let customer: BrailsCustomer = state
        .brails_client
        .get_customer(&api_key, &customer_id)
        .await
        .map_err(AppError::ProviderError)?;

    info!(user_id = %claims.sub, customer_id = %customer_id, "Fetched Brails customer");

    Ok((StatusCode::OK, Json(customer)))
}

/// PUT /api/v1/brails/customers/{id}
pub async fn update_customer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(customer_id): Path<String>,
    Json(payload): Json<UpdateCustomerRequest>,
) -> Result<impl IntoResponse, AppError> {
    let api_key = get_api_key(&state).await?;

    let brails_payload = BrailsUpdateCustomerPayload {
        email: payload.email,
        first_name: payload.first_name,
        last_name: payload.last_name,
        phone: payload.phone,
        country_code: payload.country_code,
    };

    let customer: BrailsCustomer = state
        .brails_client
        .update_customer(&api_key, &customer_id, brails_payload)
        .await
        .map_err(AppError::ProviderError)?;

    info!(user_id = %claims.sub, customer_id = %customer_id, "Updated Brails customer");

    Ok((StatusCode::OK, Json(customer)))
}

/// GET /api/v1/brails/customers
pub async fn list_customers(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let api_key = get_api_key(&state).await?;

    let customers: BrailsCustomerList = state
        .brails_client
        .list_customers(&api_key)
        .await
        .map_err(AppError::ProviderError)?;

    info!(user_id = %claims.sub, "Listed Brails customers");

    Ok((StatusCode::OK, Json(customers)))
}
