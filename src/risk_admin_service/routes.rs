use crate::risk_admin_service::handlers;
use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn risk_admin_router() -> Router<AppState> {
    Router::new()
        .route("/funding-events", get(handlers::list_held_funding_events))
        .route(
            "/funding-events/:id/approve",
            post(handlers::approve_funding_event),
        )
        .route("/fraud-alerts", get(handlers::list_fraud_alerts))
}
