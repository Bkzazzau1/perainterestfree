use crate::wallet_service::handlers;
use crate::AppState;
use axum::{routing::get, Router};

/// Router for all wallet-related endpoints
pub fn wallet_router() -> Router<AppState> {
    Router::new()
        .route("/wallets/summary", get(handlers::get_wallet_summary))
        .route("/wallets/transactions", get(handlers::get_transactions))
    // REMOVED: .route_layer(...)
}
