use axum::{
    routing::{get, post},
    Router,
};
use crate::AppState;
use crate::admin_settings_service::handlers;
use crate::admin_auth_service::middleware::admin_auth_middleware;
use axum::middleware;

/// Router for /api/v1/admin/settings
pub fn admin_settings_router() -> Router<AppState> {
    Router::new()
        .route("/settings", get(handlers::get_settings))
        .route("/settings", post(handlers::update_settings))
        // This middleware will be updated to check for 'super_admin' role
        .route_layer(middleware::from_fn_with_state(
            admin_auth_middleware,
        ))
}