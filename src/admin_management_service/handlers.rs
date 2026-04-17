use crate::admin_auth_service::{
    extractors::AdminPermissions, // <-- Our new extractor
    models::AdminClaims,
};
use crate::admin_management_service::models::{AssignRolePayload, CreateRolePayload};
use crate::{error::AppError, AppState};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde_json::json; // <-- Need this for the response
use tracing::info;

// Helper to check permission
fn require_permission(perms: &AdminPermissions, key: &str) -> Result<(), AppError> {
    if !perms.0.contains(key) {
        Err(AppError::Unauthorized)
    } else {
        Ok(())
    }
}

/// Handler for GET /api/v1/admin/management/roles
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn list_roles(
    State(state): State<AppState>,
    Extension(claims): Extension<AdminClaims>,
    perms: AdminPermissions, // <-- Use the extractor
) -> Result<impl IntoResponse, AppError> {
    // 1. Check permission
    require_permission(&perms, "admin:create_role")?; // Re-using this perm

    // 2. Fetch all roles and their permissions
    let records = sqlx::query!(
        r#"
        SELECT
            r.id as role_id,
            r.role_name,
            ARRAY_AGG(p.permission_key) as permissions
        FROM admin_roles r
        LEFT JOIN admin_role_permissions rp ON r.id = rp.role_id
        LEFT JOIN admin_permissions p ON rp.permission_id = p.id
        GROUP BY r.id, r.role_name
        ORDER BY r.role_name
        "#
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // 3. Format the response
    let roles: Vec<_> = records
        .into_iter()
        .map(|r| {
            json!({
                "id": r.role_id,
                "role_name": r.role_name,
                // ARRAY_AGG returns [null] for empty, so we handle it
                "permissions": r.permissions.unwrap_or(vec![])
            })
        })
        .collect();

    info!(admin_id = %claims.sub, "Viewed all roles");
    Ok((StatusCode::OK, Json(roles)))
}

/// Handler for GET /api/v1/admin/management/permissions
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn list_permissions(
    State(state): State<AppState>,
    Extension(_claims): Extension<AdminClaims>,
    perms: AdminPermissions,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&perms, "admin:create_role")?;

    let perms = sqlx::query!("SELECT permission_key, description FROM admin_permissions")
        .fetch_all(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?;

    let result: Vec<_> = perms
        .into_iter()
        .map(|p| json!({ "key": p.permission_key, "description": p.description }))
        .collect();

    Ok((StatusCode::OK, Json(result)))
}

/// Handler for POST /api/v1/admin/management/roles
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn create_role(
    State(state): State<AppState>,
    Extension(claims): Extension<AdminClaims>,
    perms: AdminPermissions,
    Json(payload): Json<CreateRolePayload>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&perms, "admin:create_role")?;

    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(AppError::DatabaseError)?;

    // 1. Create the new role
    let new_role = sqlx::query!(
        "INSERT INTO admin_roles (role_name, description) VALUES ($1, $2) RETURNING id",
        payload.role_name,
        payload.description
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // 2. Get IDs for the provided permission keys
    let perm_ids = sqlx::query!(
        "SELECT id FROM admin_permissions WHERE permission_key = ANY($1)",
        &payload.permissions
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    // 3. Insert into the join table
    for perm in perm_ids {
        sqlx::query!(
            "INSERT INTO admin_role_permissions (role_id, permission_id) VALUES ($1, $2)",
            new_role.id,
            perm.id
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;
    }

    tx.commit().await.map_err(AppError::DatabaseError)?;

    info!(admin_id = %claims.sub, new_role = %payload.role_name, "Created new admin role");
    Ok((StatusCode::CREATED, "Role created successfully"))
}

/// Handler for POST /api/v1/admin/management/users/assign-role
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn assign_role(
    State(state): State<AppState>,
    Extension(claims): Extension<AdminClaims>,
    perms: AdminPermissions,
    Json(payload): Json<AssignRolePayload>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&perms, "admin:assign_role")?;

    sqlx::query!(
        "UPDATE admin_users SET role_id = $1 WHERE id = $2",
        payload.role_id,
        payload.user_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    info!(admin_id = %claims.sub, user_id = %payload.user_id, role_id = %payload.role_id, "Assigned admin role");
    Ok((StatusCode::OK, "Role assigned"))
}
