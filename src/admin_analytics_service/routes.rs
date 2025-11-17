use axum::{
    routing::{get},
    Router,
};
use crate::AppState;
use crate::admin_analytics_service::handlers;

/// Router for /api/v1/admin/analytics/*
pub fn analytics_router() -> Router<AppState> {
    Router::new()
        .route("/analytics/stats", get(handlers::get_dashboard_stats))
        // We'll add the dormancy report export here later
        // .route("/reports/dormancy", get(handlers::get_dormancy_report))
}