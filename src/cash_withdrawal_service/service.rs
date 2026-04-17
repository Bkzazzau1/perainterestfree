use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::security::{hash_value, verify_value};
use crate::cash_withdrawal_service::models::PartnerContext;
use crate::error::AppError;

pub const SUPPORTED_CURRENCIES: [&str; 2] = ["USD", "GBP"];
pub const SUPPORTED_METHODS: [&str; 2] = ["PICKUP", "DELIVERY"];
pub const DEFAULT_PICKUP_EXPIRY_HOURS: i64 = 48;
const FEE_BPS: i64 = 100; // 1%
const MIN_FEE_MINOR: i64 = 500; // placeholder flat minimum fee
const MAX_FAILED_ATTEMPTS: i32 = 5;
const ATTEMPT_COOLDOWN_MINUTES: i64 = 15;

pub fn normalize_uppercase(input: &str) -> String {
    input.trim().to_uppercase()
}

pub fn validate_currency(currency: &str) -> Result<(), AppError> {
    if SUPPORTED_CURRENCIES.contains(&currency) {
        Ok(())
    } else {
        Err(AppError::ProviderError("UNSUPPORTED_CURRENCY".to_string()))
    }
}

pub fn validate_method(method: &str) -> Result<(), AppError> {
    if SUPPORTED_METHODS.contains(&method) {
        Ok(())
    } else {
        Err(AppError::ProviderError("UNSUPPORTED_METHOD".to_string()))
    }
}

pub fn amount_to_minor(amount: f64) -> Result<i64, AppError> {
    let minor = (amount * 100.0).round() as i64;
    if minor <= 0 {
        return Err(AppError::ProviderError("INVALID_AMOUNT".to_string()));
    }
    Ok(minor)
}

pub fn compute_quote(amount_minor: i64) -> (i64, i64) {
    let fee = ((amount_minor as f64) * (FEE_BPS as f64) / 10_000.0).round() as i64;
    let fee_minor = fee.max(MIN_FEE_MINOR);
    let total_debit_minor = amount_minor + fee_minor;
    (fee_minor, total_debit_minor)
}

pub async fn ensure_ng_and_kyc(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    let profile = sqlx::query!(
        r#"
        SELECT country, bvn_encrypted, nin_encrypted
        FROM user_profiles
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::KycIncomplete)?;

    if profile.country.unwrap_or_default().to_uppercase() != "NG" {
        return Err(AppError::ProviderError("NG_ONLY".to_string()));
    }

    if profile.bvn_encrypted.is_none() && profile.nin_encrypted.is_none() {
        return Err(AppError::KycIncomplete);
    }

    Ok(())
}

pub async fn ensure_pin(pool: &PgPool, user_id: Uuid, pin: String) -> Result<(), AppError> {
    let user = sqlx::query!("SELECT pin_hash FROM users WHERE id = $1", user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::Unauthorized)?; // user missing

    let pin_hash = user
        .pin_hash
        .ok_or(AppError::ProviderError("PIN_NOT_SET".to_string()))?;

    let valid = verify_value(pin, pin_hash).await?;
    if !valid {
        return Err(AppError::InvalidCredentials);
    }

    Ok(())
}

pub fn generate_reference() -> String {
    Uuid::new_v4().to_string()
}

pub async fn generate_pickup_code() -> Result<(String, String), AppError> {
    let code: u32 = rand::thread_rng().gen_range(0..=999_999);
    let code_str = format!("{:06}", code);
    let hash = hash_value(code_str.clone()).await?;
    Ok((code_str, hash))
}

pub async fn create_hold_and_transaction(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    currency: &str,
    total_debit_minor: i64,
    reference: &str,
) -> Result<(Uuid, Uuid, Uuid), AppError> {
    let wallet = sqlx::query!(
        r#"
        SELECT id, balance_minor
        FROM wallets
        WHERE user_id = $1 AND currency = $2
        FOR UPDATE
        "#,
        user_id,
        currency
    )
    .fetch_optional(tx.as_mut())
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or(AppError::ProviderError("WALLET_NOT_FOUND".to_string()))?;

    if wallet.balance_minor < total_debit_minor {
        return Err(AppError::ProviderError("INSUFFICIENT_FUNDS".to_string()));
    }

    sqlx::query!(
        r#"
        UPDATE wallets
        SET balance_minor = balance_minor - $1, updated_at = NOW()
        WHERE id = $2
        "#,
        total_debit_minor,
        wallet.id
    )
    .execute(tx.as_mut())
    .await
    .map_err(AppError::DatabaseError)?;

    let transaction = sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title, reference, metadata
        )
        VALUES ($1, $2, 'cash_withdrawal', 'pending', $3, $4, $5, $6, $7)
        RETURNING id
        "#,
        user_id,
        wallet.id,
        -total_debit_minor, // debit
        currency,
        format!("Cash withdrawal {}", currency),
        reference,
        json!({"reference": reference})
    )
    .fetch_one(tx.as_mut())
    .await
    .map_err(AppError::DatabaseError)?;

    let hold = sqlx::query!(
        r#"
        INSERT INTO wallet_holds (
            user_id, wallet_id, amount_minor, currency, reason, status, reference
        )
        VALUES ($1, $2, $3, $4, 'cash_withdrawal', 'HELD', $5)
        RETURNING id
        "#,
        user_id,
        wallet.id,
        total_debit_minor,
        currency,
        reference
    )
    .fetch_one(tx.as_mut())
    .await
    .map_err(AppError::DatabaseError)?;

    Ok((wallet.id, transaction.id, hold.id))
}

pub async fn log_audit(
    pool: &PgPool,
    actor_user_id: Option<Uuid>,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
    metadata: Value,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        actor_user_id,
        action,
        entity_type,
        entity_id,
        metadata
    )
    .execute(pool)
    .await
    .map_err(AppError::DatabaseError)?;
    Ok(())
}

pub async fn ensure_partner_role(
    pool: &PgPool,
    user_id: Uuid,
    required_role: &str,
) -> Result<PartnerContext, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT pu.partner_org_id, pu.role
        FROM partner_users pu
        JOIN partner_organizations po ON po.id = pu.partner_org_id
        WHERE pu.user_id = $1
          AND pu.is_active = TRUE
          AND po.status = 'APPROVED'
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::DatabaseError)?;

    let mut matched: Option<Uuid> = None;
    for row in rows {
        let role = row.role;
        if role == required_role || role == "PARTNER_ADMIN" {
            matched = Some(row.partner_org_id);
            break;
        }
    }

    if let Some(org_id) = matched {
        Ok(PartnerContext {
            user_id,
            partner_org_id: org_id,
        })
    } else {
        Err(AppError::Forbidden)
    }
}

pub fn pickup_expiry(hours: Option<i64>) -> DateTime<Utc> {
    let hrs = hours.unwrap_or(DEFAULT_PICKUP_EXPIRY_HOURS).max(1);
    Utc::now() + Duration::hours(hrs)
}

pub fn should_rate_limit(failed_attempts: i32, last_failed_at: Option<DateTime<Utc>>) -> bool {
    if failed_attempts < MAX_FAILED_ATTEMPTS {
        return false;
    }
    if let Some(ts) = last_failed_at {
        let window = Utc::now() - Duration::minutes(ATTEMPT_COOLDOWN_MINUTES);
        return ts > window;
    }
    false
}

pub async fn refund_hold_and_wallet(
    tx: &mut Transaction<'_, Postgres>,
    withdrawal_reference: &str,
    hold_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    // Fetch hold and related wallet
    let hold = sqlx::query!(
        r#"
        SELECT wallet_id, amount_minor, currency, status
        FROM wallet_holds
        WHERE id = $1
        FOR UPDATE
        "#,
        hold_id
    )
    .fetch_one(tx.as_mut())
    .await
    .map_err(AppError::DatabaseError)?;

    if hold.status == "RELEASED" || hold.status == "CONSUMED" {
        return Ok(());
    }

    sqlx::query!(
        r#"
        UPDATE wallets
        SET balance_minor = balance_minor + $1, updated_at = NOW()
        WHERE id = $2
        "#,
        hold.amount_minor,
        hold.wallet_id
    )
    .execute(tx.as_mut())
    .await
    .map_err(AppError::DatabaseError)?;

    sqlx::query!(
        r#"
        INSERT INTO transactions (
            user_id, wallet_id, type, status, amount_minor, currency, title, reference, metadata
        )
        VALUES ($1, $2, 'cash_withdrawal_refund', 'completed', $3, $4, $5, $6, $7)
        "#,
        user_id,
        hold.wallet_id,
        hold.amount_minor,
        hold.currency,
        "Cash withdrawal refund",
        withdrawal_reference,
        json!({"reference": withdrawal_reference})
    )
    .execute(tx.as_mut())
    .await
    .map_err(AppError::DatabaseError)?;

    sqlx::query!(
        r#"
        UPDATE wallet_holds
        SET status = 'RELEASED', updated_at = NOW(), released_at = NOW()
        WHERE id = $1
        "#,
        hold_id
    )
    .execute(tx.as_mut())
    .await
    .map_err(AppError::DatabaseError)?;

    Ok(())
}
