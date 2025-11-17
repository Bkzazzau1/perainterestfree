use axum::{
    routing::{get, post},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::crypto_service::handlers;

/// Router for all crypto endpoints
pub fn crypto_router() -> Router<AppState> {
    Router::new()
        .route("/crypto/addresses", get(handlers::get_receive_address))
        // 'get_quote' is now handled by the 'convert_service' router
        // .route("/crypto/quote", get(handlers::get_quote)) // <-- REMOVED
        .route("/crypto/send", post(handlers::send_crypto))
        // 'convert_assets' is now handled by the 'convert_service' router
        // .route("/crypto/convert", post(handlers::convert_assets)) // <-- REMOVED
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}