use crate::payment_service::handlers;
use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

/// Router for all payment (P2P/payout) endpoints
pub fn payment_router() -> Router<AppState> {
    Router::<AppState>::new()
        .route(
            "/payments/country-matrix",
            get(handlers::get_country_matrix),
        )
        .route("/payments/transfer", post(handlers::perform_transfer))
    // REMOVED: .route_layer(...)
}
