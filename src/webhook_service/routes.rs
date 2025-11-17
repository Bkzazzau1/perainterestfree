use axum::{
    routing::{post},
    Router,
    middleware,
};
use crate::AppState;
use crate::webhook_service::{handlers, middleware::brails_auth};

/// Router for all incoming webhooks (public, custom auth)
pub fn webhook_router() -> Router<AppState> {
    Router::new()
        // Webhook for card deposits
        .route("/api/v1/webhooks/brails/deposit", post(handlers::brails_deposit))
        // Webhook for real-time card authorization
        .route(
            "/api/v1/webhooks/brails/card-auth",
            post(handlers::brails_card_auth),
        )
        .route_layer(middleware::from_fn_with_state(brails_auth))
}