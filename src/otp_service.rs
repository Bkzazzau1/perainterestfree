use chrono::{Duration, Utc};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::security::{hash_value, verify_value};
use crate::error::AppError;

const OTP_EXPIRY_MINUTES: i64 = 10;

pub fn normalize_channel(channel: &str) -> String {
    channel.trim().to_uppercase()
}

pub fn normalize_target(channel: &str, target: &str) -> String {
    if channel == "EMAIL" {
        target.trim().to_lowercase()
    } else {
        target.trim().to_string()
    }
}

pub async fn create_otp(
    pool: &PgPool,
    user_id: Option<Uuid>,
    purpose: &str,
    channel: &str,
    target: &str,
) -> Result<String, AppError> {
    let code = format!("{:06}", rand::thread_rng().gen_range(0..=999_999));
    let code_hash = hash_value(code.clone()).await?;
    let expires_at = Utc::now() + Duration::minutes(OTP_EXPIRY_MINUTES);

    sqlx::query!(
        r#"
        DELETE FROM verification_otps
        WHERE purpose = $1 AND channel = $2 AND target = $3
          AND (($4::UUID IS NULL AND user_id IS NULL) OR user_id = $4)
        "#,
        purpose,
        channel,
        target,
        user_id
    )
    .execute(pool)
    .await
    .map_err(AppError::DatabaseError)?;

    sqlx::query!(
        r#"
        INSERT INTO verification_otps (user_id, purpose, channel, target, code_hash, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        user_id,
        purpose,
        channel,
        target,
        code_hash,
        expires_at
    )
    .execute(pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(code)
}

pub async fn mark_verified(
    pool: &PgPool,
    user_id: Option<Uuid>,
    purpose: &str,
    channel: &str,
    target: &str,
    code: &str,
) -> Result<(), AppError> {
    let row = sqlx::query!(
        r#"
        SELECT id, code_hash
        FROM verification_otps
        WHERE purpose = $1 AND channel = $2 AND target = $3
          AND (($4::UUID IS NULL AND user_id IS NULL) OR user_id = $4)
          AND verified_at IS NULL
          AND expires_at > NOW()
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        purpose,
        channel,
        target,
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::InvalidCredentials)?;

    let valid = verify_value(code.to_string(), row.code_hash).await?;
    if !valid {
        return Err(AppError::InvalidCredentials);
    }

    sqlx::query!(
        "UPDATE verification_otps SET verified_at = NOW() WHERE id = $1",
        row.id
    )
    .execute(pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(())
}

pub async fn require_verified(
    pool: &PgPool,
    user_id: Option<Uuid>,
    purpose: &str,
    channel: &str,
    target: &str,
) -> Result<(), AppError> {
    let row = sqlx::query!(
        r#"
        SELECT id
        FROM verification_otps
        WHERE purpose = $1 AND channel = $2 AND target = $3
          AND (($4::UUID IS NULL AND user_id IS NULL) OR user_id = $4)
          AND verified_at IS NOT NULL
          AND expires_at > NOW()
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        purpose,
        channel,
        target,
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::DatabaseError)?;

    if row.is_none() {
        return Err(AppError::ProviderError("OTP_NOT_VERIFIED".to_string()));
    }

    Ok(())
}

pub async fn consume(
    pool: &PgPool,
    user_id: Option<Uuid>,
    purpose: &str,
    channel: &str,
    target: &str,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        DELETE FROM verification_otps
        WHERE purpose = $1 AND channel = $2 AND target = $3
          AND (($4::UUID IS NULL AND user_id IS NULL) OR user_id = $4)
        "#,
        purpose,
        channel,
        target,
        user_id
    )
    .execute(pool)
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(())
}
