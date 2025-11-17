use axum::{
    routing::{get, post, put},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::card_service::handlers;

/// Router for all card management endpoints
pub fn card_router() -> Router<AppState> {
    Router::new()
        .route("/cards", get(handlers::get_cards))
        .route("/cards/virtual", post(handlers::create_virtual_card))
        .route("/cards/physical", post(handlers::request_physical_card))
        .route("/cards/:card_id/freeze", post(handlers::freeze_card))
        .route("/cards/:card_id/unfreeze", post(handlers::unfreeze_card))
        .route("/cards/:card_id/toggles", put(handlers::set_card_toggles))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}