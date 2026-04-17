use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
#[allow(dead_code)]
pub struct RoleWithPermissions {
    pub id: Uuid,
    pub role_name: String,
    pub permissions: Vec<String>,
}

#[derive(Deserialize)]
pub struct CreateRolePayload {
    pub role_name: String,
    pub description: String,
    pub permissions: Vec<String>, // List of permission_key strings
}

#[derive(Deserialize)]
pub struct AssignRolePayload {
    pub user_id: Uuid,
    pub role_id: Uuid,
}
