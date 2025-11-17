use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct RoleWithPermissions {
    id: Uuid,
    role_name: String,
    permissions: Vec<String>,
}

#[derive(Deserialize)]
pub struct CreateRolePayload {
    role_name: String,
    description: String,
    permissions: Vec<String>, // List of permission_key strings
}

#[derive(Deserialize)]
pub struct AssignRolePayload {
    user_id: Uuid,
    role_id: Uuid,
}