use crate::admin_auth_service::models::AdminClaims;
use crate::{error::AppError, AppState};
use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};

/// Middleware to protect all /admin/* routes
#[allow(dead_code)]
pub async fn admin_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. Extract the token
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    // 2. Decode the token using AdminClaims
    let key = DecodingKey::from_secret(state.jwt_secret.as_bytes());
    let claims = decode::<AdminClaims>(token, &key, &Validation::default())
        .map(|data| data.claims)
        .map_err(|_| AppError::Unauthorized)?; // Use 401, not 500

    // 3. (Optional) Check role-based access
    // if claims.role != "super_admin" {
    //     return Err(AppError::Unauthorized);
    // }

    // 4. Insert claims for the handler
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
