// FILE: src/routes/bills.rs

use axum::{
    routing::{get, post},
    Router,
};

use crate::bills_service::handlers;
use crate::AppState;

/// Router for all bill payment endpoints
///
/// NOTE:
/// - Electricity is supported through the generic endpoints:
///   - GET  /bills/services?category=electricity
///   - POST /bills/validate   (category="electricity")
///   - POST /bills/pay        (category="electricity")
///
/// We do NOT need separate /bills/electricity/... routes unless you want
/// provider-specific lookups (e.g., disco list) exposed as dedicated endpoints.
pub fn bills_router() -> Router<AppState> {
    Router::new()
        // ---- Generic Bills API (covers electricity, cable_tv, airtime, data, etc.) ----
        .route("/bills/services", get(handlers::get_services))
        .route("/bills/validate", post(handlers::validate_customer))
        .route("/bills/pay", post(handlers::pay_bill))
        // ---- Cable TV helpers (Payscribe direct lookups) ----
        .route("/bills/cable/bouquets", get(handlers::get_cable_bouquets))
        // ---- ePins ----
        .route("/bills/epins", get(handlers::get_epins))
        .route("/bills/epins/vend", post(handlers::vend_epin))

    // OPTIONAL (recommended next):
    // Add these only if you implemented handlers for them.
    //
    // ---- Data Bundles (lookup plans) ----
    // .route("/bills/data/plans", get(handlers::get_data_plans))
    //
    // ---- Electricity helper lookups (DISCOs / packages) ----
    // .route("/bills/electricity/providers", get(handlers::get_electricity_providers))
    //
    // ---- Airtime helper lookup (networks) ----
    // .route("/bills/airtime/networks", get(handlers::get_airtime_networks))
}
