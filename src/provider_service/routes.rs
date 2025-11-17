use axum::{
    routing::{post},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::provider_service::handlers;

/// Router for all external provider-related endpoints
pub fn provider_router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/accounts/create", post(handlers::create_virtual_account))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}