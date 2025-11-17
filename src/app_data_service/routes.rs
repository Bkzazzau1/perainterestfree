use axum::{routing::get, Router};
use crate::AppState;
use crate::app_data_service::handlers;

/// Router for public app configuration data
pub fn app_data_router() -> Router<AppState> {
    Router::new()
        .route("/app/config", get(handlers::get_app_config))
}