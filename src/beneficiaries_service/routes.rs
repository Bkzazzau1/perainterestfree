use axum::{
    routing::{get, post, put, delete},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::beneficiaries_service::handlers;

/// Router for all beneficiaries CRUD endpoints
pub fn beneficiaries_router() -> Router<AppState> {
    Router::new()
        .route("/beneficiaries", get(handlers::get_beneficiaries))
        .route("/beneficiaries", post(handlers::add_beneficiary))
        .route("/beneficiaries/:id", put(handlers::update_beneficiary))
        .route("/beneficiaries/:id", delete(handlers::delete_beneficiary))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}