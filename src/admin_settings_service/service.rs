use crate::error::AppError;
use sqlx::PgPool;
use std::collections::HashMap;

/// Fetches all system settings as a HashMap
pub async fn get_all_settings(pool: &PgPool) -> Result<HashMap<String, String>, AppError> {
    let records = sqlx::query!("SELECT key, value FROM system_settings")
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)?;

    let settings = records.into_iter().map(|r| (r.key, r.value)).collect();
    Ok(settings)
}
