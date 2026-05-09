//! Two-Factor Authentication repository — TOTP secrets and backup codes.
//!
//! Secrets are stored as base32 strings.  The caller (nexus-api) is
//! responsible for any additional encryption layer before storage.

use sha2::{Digest, Sha256};
use uuid::Uuid;

// ── TOTP secret management ────────────────────────────────────────────────────

/// Persist the pending TOTP secret for a user (not yet enabled).
pub async fn set_totp_secret(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    secret_b32: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET totp_secret = $1, totp_enabled = false WHERE id = $2::uuid")
        .bind(secret_b32)
        .bind(user_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Retrieve the pending/active TOTP secret for a user.
pub async fn get_totp_secret(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT totp_secret FROM users WHERE id = $1::uuid")
            .bind(user_id.to_string())
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|(s,)| s))
}

/// Enable TOTP on the account (marks totp_enabled = true).
/// Call only after the user has verified their first TOTP code.
pub async fn enable_totp(pool: &sqlx::AnyPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET totp_enabled = true WHERE id = $1::uuid")
        .bind(user_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Disable TOTP on the account and clear the stored secret.
pub async fn disable_totp(pool: &sqlx::AnyPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET totp_enabled = false, totp_secret = NULL WHERE id = $1::uuid")
        .bind(user_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

// ── Backup codes ──────────────────────────────────────────────────────────────

/// Hash a raw 8-character backup code with SHA-256 for safe storage.
pub fn hash_backup_code(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

/// Delete all existing backup codes for the user and insert fresh ones.
pub async fn replace_backup_codes(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    code_hashes: &[String],
) -> Result<(), sqlx::Error> {
    // Remove old codes
    sqlx::query("DELETE FROM totp_backup_codes WHERE user_id = $1::uuid")
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

    // Insert new codes
    for hash in code_hashes {
        sqlx::query(
            "INSERT INTO totp_backup_codes (id, user_id, code_hash) \
             VALUES (gen_random_uuid(), $1::uuid, $2)",
        )
        .bind(user_id.to_string())
        .bind(hash)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Count unused backup codes remaining for a user.
pub async fn count_unused_backup_codes(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM totp_backup_codes WHERE user_id = $1::uuid AND used_at IS NULL",
    )
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Try to consume a backup code by its raw value (SHA-256 lookup).
///
/// Returns `true` if a matching unused code was found and marked used.
pub async fn consume_backup_code(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    raw_code: &str,
) -> Result<bool, sqlx::Error> {
    let hash = hash_backup_code(raw_code);
    let result = sqlx::query(
        "UPDATE totp_backup_codes \
         SET used_at = NOW() \
         WHERE user_id = $1::uuid \
           AND code_hash = $2 \
           AND used_at IS NULL",
    )
    .bind(user_id.to_string())
    .bind(&hash)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
