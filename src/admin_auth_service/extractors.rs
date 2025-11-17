use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::request::Parts,
};
use std::collections::HashSet;
use crate::{error::AppError, AppState};
use crate::admin_auth_service::models::AdminClaims;
use tracing::warn;

/// An extractor that checks if the logged-in admin has a specific permission.
/// Use as: `async fn handler(RequirePermission(p): RequirePermission, ...)`
pub struct RequirePermission(pub String);

#[async_trait]
impl FromRequestParts<AppState> for RequirePermission {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        
        // 1. Get the claims (which must have been inserted by the middleware)
        let claims = parts.extensions.get::<AdminClaims>()
            .ok_or_else(|| {
                warn!("RequirePermission extractor ran before admin_auth_middleware");
                AppError::InternalServerError
            })?;
        
        // 2. Get the list of permissions for this admin's role
        let permissions = get_permissions_for_role(&state.db_pool, &claims.role).await?;
        
        // 3. Get the required permission from the handler's signature
        // This is a bit of a placeholder as we can't get the string "admin:create_role"
        // directly from the type. We'll modify this logic.
        // ---
        // **Correction**: The extractor logic is simpler.
        // We will create *another* extractor that *just* loads the permissions.
        // Then, a handler can use `RequirePermission`
        
        // Let's restart this extractor. It's simpler.
        // It just requires a *list* of permissions.
        
        Err(AppError::InternalServerError) // Placeholder
    }
}


/// --- REVISED EXTRACTOR ---
/// This extractor just fetches the admin's permissions.
#[derive(Clone, Debug)]
pub struct AdminPermissions(pub HashSet<String>);

#[async_trait]
impl FromRequestParts<AppState> for AdminPermissions {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = parts.extensions.get::<AdminClaims>()
            .ok_or(AppError::InternalServerError)?; // Middleware should run first
            
        let perms_set = get_permissions_for_role(&state.db_pool, &claims.role).await?;
        Ok(AdminPermissions(perms_set))
    }
}

/// Helper function to get all permission keys for a given role name
async fn get_permissions_for_role(
    pool: &sqlx::PgPool,
    role_name: &str,
) -> Result<HashSet<String>, AppError> {
    let perms = sqlx::query!(
        r#"
        SELECT p.permission_key
        FROM admin_permissions p
        JOIN admin_role_permissions rp ON p.id = rp.permission_id
        JOIN admin_roles r ON rp.role_id = r.id
        WHERE r.role_name = $1
        "#,
        role_name
    )
    .fetch_all(pool)
    .await.map_err(AppError::DatabaseError)?
    .into_iter()
    .map(|rec| rec.permission_key)
    .collect();
    
    Ok(perms)
}