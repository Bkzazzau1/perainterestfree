use crate::user_admin_service::handlers;
use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn user_admin_router() -> Router<AppState> {
    Router::new()
        .route("/users", get(handlers::list_users))
        .route("/users/:id", get(handlers::get_user_detail))
        .route("/users/:id/kyc", post(handlers::update_kyc_status))
}
