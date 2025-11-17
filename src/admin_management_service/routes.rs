use axum::{
    routing::{get, post},
    Router,
};
use crate::AppState;
use crate::admin_management_service::handlers;

/// Router for /api/v1/admin/management/*
pub fn admin_management_router() -> Router<AppState> {
    Router::new()
        .route("/roles", get(handlers::list_roles))
        .route("/roles", post(handlers::create_role))
        .route("/permissions", get(handlers::list_permissions))
        .route("/users/assign-role", post(handlers::assign_role))
}