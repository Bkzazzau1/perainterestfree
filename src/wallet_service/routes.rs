use axum::{
    routing::{get},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::wallet_service::handlers;

/// Router for all wallet-related endpoints
pub fn wallet_router() -> Router<AppState> {
    Router::new()
        .route("/wallets/summary", get(handlers::get_wallet_summary))
        .route("/wallets/transactions", get(handlers::get_transactions))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}