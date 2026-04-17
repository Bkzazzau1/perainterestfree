use crate::admin_management_service::handlers;
use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn admin_management_router() -> Router<AppState> {
    Router::new()
        .route("/roles", get(handlers::list_roles))
        .route("/roles", post(handlers::create_role))
        .route("/permissions", get(handlers::list_permissions))
        .route("/users/assign-role", post(handlers::assign_role))
}
