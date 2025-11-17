use axum::{
    routing::{post},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::payment_service::handlers;

/// Router for all payment (P2P/payout) endpoints
pub fn payment_router() -> Router<AppState> {
    Router::new()
        // Matches 'payment_api.dart'
        .route("/payments/transfer", post(handlers::perform_transfer))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}