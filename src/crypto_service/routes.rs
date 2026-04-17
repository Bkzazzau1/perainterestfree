use crate::crypto_service::handlers;
use crate::AppState;
use axum::{routing::post, Router};

/// Router for all crypto endpoints
pub fn crypto_router() -> Router<AppState> {
    Router::new()
        .route(
            "/crypto/deposit-address/:asset/:chain",
            post(handlers::get_receive_address),
        )
        .route("/crypto/send", post(handlers::send_crypto))
    // REMOVED: .route_layer(...)
}
