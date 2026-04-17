use crate::admin_analytics_service::handlers;
use crate::AppState;
use axum::{routing::get, Router};

pub fn analytics_router() -> Router<AppState> {
    Router::new()
        .route("/analytics/stats", get(handlers::get_dashboard_stats))
        .route("/reports/dormancy", get(handlers::get_dormancy_report))
}
