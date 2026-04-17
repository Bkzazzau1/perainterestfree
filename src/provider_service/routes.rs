use crate::provider_service::handlers;
use crate::AppState;
use axum::{routing::post, Router};

/// Router for all external provider-related endpoints
pub fn provider_router() -> Router<AppState> {
    Router::new().route(
        "/api/v1/accounts/create",
        post(handlers::create_virtual_account),
    )
    // REMOVED: .route_layer(...)
}
