use axum::{
    routing::{get, post},
    Router,
};

use crate::cash_deposit_service::handlers;
use crate::AppState;

pub fn cash_deposit_router() -> Router<AppState> {
    Router::new()
        .route("/cash-deposit/config", get(handlers::get_config))
        .route("/cash-deposit", post(handlers::create_deposit))
        .route("/cash-deposit/history", get(handlers::get_history))
        .route("/cash-deposit/:reference", get(handlers::get_deposit))
}

pub fn partner_router() -> Router<AppState> {
    Router::new()
        .route("/partner/bdc/deposits", get(handlers::partner_list))
        .route(
            "/partner/bdc/deposits/:reference/accept",
            post(handlers::partner_accept),
        )
        .route(
            "/partner/bdc/deposits/:reference/complete",
            post(handlers::partner_complete),
        )
        .route(
            "/partner/bdc/deposits/:reference/reject",
            post(handlers::partner_reject),
        )
}
