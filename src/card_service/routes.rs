use crate::card_service::handlers;
use crate::AppState;
use axum::{
    routing::{get, post, put},
    Router,
};

pub fn card_router() -> Router<AppState> {
    Router::new()
        .route("/cards/users/register", post(handlers::register_card_user))
        .route("/cards", get(handlers::get_cards))
        .route("/cards/virtual", post(handlers::create_virtual_card))
        .route("/cards/physical", post(handlers::request_physical_card))
        .route("/cards/:card_id/freeze", post(handlers::freeze_card))
        .route("/cards/:card_id/unfreeze", post(handlers::unfreeze_card))
        .route("/cards/:card_id/toggles", put(handlers::set_card_toggles))
}
