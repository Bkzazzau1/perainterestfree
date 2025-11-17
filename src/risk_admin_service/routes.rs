use axum::{
    routing::{get, post},
    Router,
};
use crate::AppState;
use crate::risk_admin_service::handlers;

/// Router for /api/v1/admin/* risk management
pub fn risk_admin_router() -> Router<AppState> {
    Router::new()
        .route("/funding-events", get(handlers::list_held_funding_events))
        .route("/funding-events/:id/approve", post(handlers::approve_funding_event))
        .route("/fraud-alerts", get(handlers::list_fraud_alerts))
    // Note: Already protected by the admin middleware in main.rs
}