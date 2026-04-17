use crate::beneficiaries_service::handlers;
use crate::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

pub fn beneficiaries_router() -> Router<AppState> {
    Router::new()
        .route("/beneficiaries", get(handlers::get_beneficiaries))
        .route("/beneficiaries", post(handlers::add_beneficiary))
        .route("/beneficiaries/:id", put(handlers::update_beneficiary))
        .route("/beneficiaries/:id", delete(handlers::delete_beneficiary))
}
