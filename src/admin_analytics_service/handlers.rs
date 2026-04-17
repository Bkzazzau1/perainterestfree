use crate::admin_auth_service::{extractors::AdminPermissions, models::AdminClaims};
use crate::{error::AppError, AppState};
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
// --- UPDATED IMPORTS ---
use crate::admin_analytics_service::models::{
    Count, DashboardStats, DormancyQuery, DormantUserRecord, Sum,
};
// ----------------------
use chrono::{DateTime, Duration, Utc};
use csv::Writer;
use tracing::info; // <-- Added csv Writer

/// Handler for GET /api/v1/admin/analytics/stats
#[axum::debug_handler] // <-- CORE FIX APPLIED
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
        .await
        .map_err(AppError::DatabaseError)?;
    stats.total_users = total_users.total.unwrap_or(0);

    // 2. New Users Today
    let today_start = {
        let naive = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
        DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
    };
    let new_users = sqlx::query_as!(
        Count,
        "SELECT COUNT(*) as total FROM users WHERE created_at >= $1",
        today_start
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;
    stats.new_users_today = new_users.total.unwrap_or(0);

    // 3. Total NGN Volume (sum of debits)
    let vol_ngn = sqlx::query_as!(
        Sum,
        "SELECT SUM(amount_minor)::bigint as total FROM transactions WHERE currency = 'NGN' AND amount_minor < 0"
    )
    .fetch_one(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    stats.total_volume_ngn = vol_ngn.total.unwrap_or(0);

    // 4. Total USD Volume (sum of debits)
    let vol_usd = sqlx::query_as!(
        Sum,
        "SELECT SUM(amount_minor)::bigint as total FROM transactions WHERE currency = 'USD' AND amount_minor < 0"
    )
    .fetch_one(&state.db_pool)
    .await.map_err(AppError::DatabaseError)?;
    stats.total_volume_usd = vol_usd.total.unwrap_or(0);

    // 5. Dormancy Reports
    let days_7 = Utc::now() - Duration::days(7);
    let days_30 = Utc::now() - Duration::days(30);
    let days_90 = Utc::now() - Duration::days(90);

    // Note: sqlx::query_as! macro requires the SQL string to be a string literal,
    // so we cannot use a variable here. We must repeat the query string.

    let dormant_7 = sqlx::query_as!(
        Count,
        r#"
        SELECT COUNT(id) as total FROM users WHERE (
            SELECT MAX(created_at) FROM transactions WHERE user_id = users.id
        ) < $1
        OR (SELECT MAX(created_at) FROM transactions WHERE user_id = users.id) IS NULL
        "#,
        days_7
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    let dormant_30 = sqlx::query_as!(
        Count,
        r#"
        SELECT COUNT(id) as total FROM users WHERE (
            SELECT MAX(created_at) FROM transactions WHERE user_id = users.id
        ) < $1
        OR (SELECT MAX(created_at) FROM transactions WHERE user_id = users.id) IS NULL
        "#,
        days_30
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    let dormant_90 = sqlx::query_as!(
        Count,
        r#"
        SELECT COUNT(id) as total FROM users WHERE (
            SELECT MAX(created_at) FROM transactions WHERE user_id = users.id
        ) < $1
        OR (SELECT MAX(created_at) FROM transactions WHERE user_id = users.id) IS NULL
        "#,
        days_90
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    stats.dormant_users_7_days = dormant_7.total.unwrap_or(0);
    stats.dormant_users_30_days = dormant_30.total.unwrap_or(0);
    stats.dormant_users_90_days = dormant_90.total.unwrap_or(0);

    info!(admin_id = %claims.sub, "Viewed admin dashboard stats");
    Ok((StatusCode::OK, Json(stats)))
}

/// Handler for GET /api/v1/admin/reports/dormancy
#[axum::debug_handler] // <-- CORE FIX APPLIED
pub async fn get_dormancy_report(
    State(state): State<AppState>,
    Extension(claims): Extension<AdminClaims>,
    perms: AdminPermissions,
    Query(query): Query<DormancyQuery>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Add a specific permission for this, e.g., "reports:export"
    if !perms.0.contains("users:read_full") {
        return Err(AppError::Unauthorized);
    }

    let threshold_date = Utc::now() - Duration::days(query.days);

    // This query finds users whose most recent transaction
    // was *before* the threshold date, OR who have no transactions at all.
    let users = sqlx::query_as!(
        DormantUserRecord,
        r#"
        SELECT
            u.id as user_id,
            u.email,
            u.phone,
            (SELECT MAX(created_at) FROM transactions WHERE user_id = u.id) as last_transaction_at
        FROM users u
        WHERE (
            SELECT MAX(created_at) FROM transactions WHERE user_id = u.id
        ) < $1
        OR (SELECT MAX(created_at) FROM transactions WHERE user_id = u.id) IS NULL
        "#,
        threshold_date
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // --- Generate CSV ---
    let mut wtr = Writer::from_writer(vec![]);
    // Write header
    wtr.serialize(("user_id", "email", "phone", "last_transaction_at"))
        .map_err(|_e| AppError::InternalServerError)?; // Simplified error

    // Write records
    for user in users {
        wtr.serialize((
            user.user_id.to_string(),
            user.email,
            user.phone,
            user.last_transaction_at
                .map_or_else(|| "".to_string(), |d| d.to_rfc3339()),
        ))
        .map_err(|_e| AppError::InternalServerError)?;
    }

    let csv_data = String::from_utf8(
        wtr.into_inner()
            .map_err(|_e| AppError::InternalServerError)?,
    )
    .map_err(|_e| AppError::InternalServerError)?;
    // --------------------

    info!(admin_id = %claims.sub, days = query.days, "Exported dormancy report");

    // Return the CSV data with headers to force a download
    let filename = format!("dormancy_report_{}_days.csv", query.days);
    let headers = [
        (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        ),
    ];

    Ok((headers, csv_data))
}
