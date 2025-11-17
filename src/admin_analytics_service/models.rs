use serde::Serialize;

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
struct Count { total: i64 }
struct Sum { total: Option<i64> }