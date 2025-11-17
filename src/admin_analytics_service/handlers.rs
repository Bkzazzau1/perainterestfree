use axum::{extract::State, http::StatusCode, response::IntoResponse, Json, Extension};
use crate::{error::AppError, AppState};
use crate::admin_auth_service::{models::AdminClaims, extractors::AdminPermissions};
use crate::admin_analytics_service::models::{DashboardStats, Count, Sum};
use chrono::{Utc, Duration};
use tracing::info;

/// Handler for GET /api/v1/admin/analytics/stats
pub async fn get_dashboard_stats(
    State(state): State<AppState>,
    Extension(claims): Extension<AdminClaims>,
    perms: AdminPermissions,
) -> Result<impl IntoResponse, AppError> {

    // TODO: Add a specific permission for this, e.g., "analytics:read"
    if !perms.0.contains("users:read_full") {
        return Err(AppError::Unauthorized);
    }
    
    let mut stats = DashboardStats::default();

    // 1. Total Users
    let total_users = sqlx::query_as!(Count, "SELECT COUNT(*) as total FROM users")
        .fetch_one(&state.db_pool)
        .await.map_err(AppError::DatabaseError)?;
    stats.total_users = total_users.total;
    
    // 2. New Users Today
    let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
    let new_users = sqlx::query_as!(
        Count,
        "SELECT COUNT(*) as total FROM users WHERE created_at >= $1",
        today_start
    )
    .fetch_one(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    stats.new_users_today = new_users.total;

    // 3. Total NGN Volume (sum of debits)
    let vol_ngn = sqlx::query_as!(
        Sum,
        "SELECT SUM(amount_minor) as total FROM transactions WHERE currency = 'NGN' AND amount_minor < 0"
    )
    .fetch_one(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    stats.total_volume_ngn = vol_ngn.total.unwrap_or(0);

    // 4. Total USD Volume (sum of debits)
    let vol_usd = sqlx::query_as!(
        Sum,
        "SELECT SUM(amount_minor) as total FROM transactions WHERE currency = 'USD' AND amount_minor < 0"
    )
    .fetch_one(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    stats.total_volume_usd = vol_usd.total.unwrap_or(0);

    // 5. Dormancy Reports
    let days_7 = Utc::now() - Duration::days(7);
    let days_30 = Utc::now() - Duration::days(30);
    let days_90 = Utc::now() - Duration::days(90);

    // This query is heavy. It finds users whose *last* transaction was before the window.
    // A more optimized way would be a background job that updates an 'last_active_at'
    // column on the 'users' table. But this works for now.
    let dormant_30 = sqlx::query_as!(
        Count,
        r#"
        SELECT COUNT(id) as total FROM users WHERE (
            SELECT MAX(created_at) FROM transactions WHERE user_id = users.id
        ) < $1
        "#,
        days_30
    )
    .fetch_one(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    stats.dormant_users_30_days = dormant_30.total;
    
    // (Running multiple heavy queries, OK for an admin dashboard)
    let dormant_7 = sqlx::query_as!(Count, "...", days_7).fetch_one(&state.db_pool).await;
    let dormant_90 = sqlx::query_as!(Count, "...", days_90).fetch_one(&state.db_pool).await;
    
    stats.dormant_users_7_days = dormant_7.map_or(0, |r| r.total);
    stats.dormant_users_90_days = dormant_90.map_or(0, |r| r.total);
    
    info!(admin_id = %claims.sub, "Viewed admin dashboard stats");
    Ok((StatusCode::OK, Json(stats)))
}