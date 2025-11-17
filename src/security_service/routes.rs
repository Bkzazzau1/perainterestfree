use axum::{
    routing::{post},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::security_service::handlers;

/// Router for all security-related endpoints
pub fn security_router() -> Router<AppState> {
    Router::new()
        .route("/security/change-password", post(handlers::change_password))
        .route("/security/set-pin", post(handlers::set_pin))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}