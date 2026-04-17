use crate::webhook_service::handlers;
use crate::AppState;
use axum::{routing::post, Router};

/// Router for all incoming webhooks
pub fn webhook_router() -> Router<AppState> {
    Router::new()
        // Brails Webhooks - Middleware will be applied in main.rs
        .route("/webhooks/brails/deposit", post(handlers::brails_deposit))
        .route(
            "/webhooks/brails/card-auth",
            post(handlers::brails_card_auth),
        )
        // Payscribe Webhook - Middleware will be applied in main.rs
        .route(
            "/webhooks/payscribe/bills",
            post(handlers::payscribe_bill_status),
        )
}
