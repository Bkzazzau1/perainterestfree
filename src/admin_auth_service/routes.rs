use crate::admin_auth_service::handlers;
use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn admin_login_router() -> Router<AppState> {
    Router::new().route("/admin/login", post(handlers::admin_login))
}

pub fn admin_protected_router() -> Router<AppState> {
    Router::new().route("/stats", get(handlers::get_admin_stats))
}
