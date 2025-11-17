use axum::{
    routing::{get, post},
    Router,
    middleware,
};
use crate::AppState;
use crate::admin_auth_service::{handlers, middleware::admin_auth_middleware};

/// Public router for the admin login endpoint
pub fn admin_login_router() -> Router<AppState> {
    Router::new()
        .route("/admin/login", post(handlers::admin_login))
}

/// Protected router for all /api/v1/admin/* endpoints
pub fn admin_protected_router() -> Router<AppState> {
    Router::new()
        .route("/stats", get(handlers::get_admin_stats))
        // (e.g., /admin/users, /admin/fraud-alerts)
        .route_layer(middleware::from_fn_with_state(
            admin_auth_middleware,
        ))
}