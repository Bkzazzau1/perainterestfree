use axum::{
    routing::{get, post},
    Router,
};

use crate::brails_customer_service::handlers;
use crate::AppState;

pub fn brails_customer_router() -> Router<AppState> {
    Router::new()
        .route(
            "/brails/customers",
            post(handlers::create_customer).get(handlers::list_customers),
        )
        .route(
            "/brails/customers/:customer_id",
            get(handlers::get_customer).put(handlers::update_customer),
        )
}
