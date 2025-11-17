use axum::{
    routing::{post},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::account_closure_service::handlers;

/// Router for the account closure endpoint
pub fn closure_router() -> Router<AppState> {
    Router::new()
        .route("/user/close-account", post(handlers::request_account_closure))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}