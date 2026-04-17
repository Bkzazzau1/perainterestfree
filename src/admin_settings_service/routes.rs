use crate::admin_settings_service::handlers;
use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn admin_settings_router() -> Router<AppState> {
    Router::new()
        .route("/settings", get(handlers::get_settings))
        .route("/settings", post(handlers::update_settings))
}
