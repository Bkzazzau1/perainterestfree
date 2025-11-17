use axum::{
    routing::{get, post},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::convert_service::handlers;

/// Router for all conversion and quote endpoints
pub fn convert_router() -> Router<AppState> {
    Router::new()
        // We'll reuse the quote logic from crypto_service
        .route("/convert/quote", get(crate::crypto_service::handlers::get_quote))
        .route("/convert/execute", post(handlers::execute_conversion))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}