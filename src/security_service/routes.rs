use crate::security_service::handlers;
use crate::AppState;
use axum::{routing::post, Router};

pub fn security_router() -> Router<AppState> {
    Router::new()
        .route("/security/change-password", post(handlers::change_password))
        .route("/security/set-pin", post(handlers::set_pin))
        .route("/security/verify-pin", post(handlers::verify_pin))
}
