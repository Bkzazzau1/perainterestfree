use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid; // <-- Added

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_users: i64,
    pub new_users_today: i64,

    // Volume is sum of debits (minor units)
    pub total_volume_ngn: i64,
    pub total_volume_usd: i64,

    // Dormancy: No transactions in X days
    pub dormant_users_7_days: i64,
    pub dormant_users_30_days: i64,
    pub dormant_users_90_days: i64,
}

// Structs for our SQL queries
pub(crate) struct Count {
    pub(crate) total: Option<i64>,
}
pub(crate) struct Sum {
    pub(crate) total: Option<i64>,
}

// --- ADDED ---

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DormancyQuery {
    // How many days of inactivity
    #[serde(default = "default_days")]
    pub days: i64,
}

fn default_days() -> i64 {
    30
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DormantUserRecord {
    pub user_id: Uuid,
    pub email: String,
    pub phone: String,
    pub last_transaction_at: Option<DateTime<Utc>>,
}
// -----------------
