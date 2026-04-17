use crate::islamic_finance_service::handlers;
use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

/// Router for all Islamic finance endpoints
pub fn islamic_router() -> Router<AppState> {
    Router::new()
        .route("/islamic/zakat-rates", get(handlers::get_zakat_rates))
        .route("/islamic/pay-zakat", post(handlers::pay_zakat))
        .route("/islamic/umrah/agencies", get(handlers::get_umrah_agencies))
        .route(
            "/islamic/umrah/agencies/resolve",
            get(handlers::resolve_umrah_agency),
        )
        .route("/islamic/umrah/receipts", get(handlers::get_umrah_receipts))
        .route("/islamic/umrah/pay", post(handlers::pay_umrah))
    // REMOVED: .route_layer(...)
}
