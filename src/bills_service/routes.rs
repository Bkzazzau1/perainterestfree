use axum::{
    routing::{get, post},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::bills_service::handlers;

/// Router for all bill payment endpoints
pub fn bills_router() -> Router<AppState> {
    Router::new()
        // Schema-related endpoints
        .route("/bills/providers", get(handlers::get_providers))
        .route("/bills/products", get(handlers::get_products))
        .route("/bills/schema", get(handlers::get_schema))
        // Payment endpoint
        .route("/bills/pay", post(handlers::pay_bill))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}