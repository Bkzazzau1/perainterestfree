use axum::{
    routing::{get, post},
    Router,
};
use crate::AppState;
use crate::user_admin_service::handlers;

/// Router for /api/v1/admin/* user management
pub fn user_admin_router() -> Router<AppState> {
    Router::new()
        .route("/users", get(handlers::list_users))
        .route("/users/:id", get(handlers::get_user_detail))
        .route("/users/:id/kyc", post(handlers::update_kyc_status))
    // Note: No route_layer is needed here because this
    // router is *nested* inside the protected admin router in main.rs
}