use axum::routing::{get, post};
use axum::Router;

use crate::cash_withdrawal_service::handlers;
use crate::AppState;

pub fn cash_withdrawal_router() -> Router<AppState> {
    Router::new()
        .route("/cash-withdrawal/config", get(handlers::get_config))
        .route("/cash-withdrawal/quote", post(handlers::get_quote))
        .route("/cash-withdrawal", post(handlers::create_withdrawal))
        .route("/cash-withdrawal/history", get(handlers::get_history))
        .route("/cash-withdrawal/:reference", get(handlers::get_withdrawal))
}

pub fn partner_router() -> Router<AppState> {
    Router::new()
        .route("/partner/bdc/withdrawals", get(handlers::partner_list))
        .route(
            "/partner/bdc/withdrawals/:reference/ready",
            post(handlers::partner_ready),
        )
        .route(
            "/partner/bdc/withdrawals/:reference/confirm",
            post(handlers::partner_confirm),
        )
        .route("/partner/travel/bookings", get(handlers::travel_list))
        .route(
            "/partner/travel/bookings/:id/confirm",
            post(handlers::travel_confirm),
        )
        .route(
            "/partner/travel/bookings/:id/delivered",
            post(handlers::travel_delivered),
        )
}
