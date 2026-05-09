//! Password reset token repository.
//!
//! Stores only SHA-256 token hashes. Raw tokens are emailed to users.

use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const TOKEN_TTL_HOURS: i64 = 2;
const TOKEN_BYTES: usize = 32;

pub fn generate_token() -> (String, String, DateTime<Utc>) {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    let raw = hex::encode(bytes);

    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let hash = hex::encode(hasher.finalize());

    let expires_at = Utc::now() + Duration::hours(TOKEN_TTL_HOURS);
    (raw, hash, expires_at)
}

fn hash_token(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn upsert_token(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1::uuid")
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query(
        "INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at) \
         VALUES (gen_random_uuid(), $1::uuid, $2, $3)",
    )
    .bind(user_id.to_string())
    .bind(token_hash)
    .bind(expires_at.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn consume_token(
    pool: &sqlx::AnyPool,
    raw_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let hash = hash_token(raw_token);

    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT id::text, user_id::text FROM password_reset_tokens \
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()",
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await?;

    let Some((token_id, user_id_str)) = row else {
        return Ok(None);
    };

    sqlx::query(
        "UPDATE password_reset_tokens \
         SET used_at = NOW() \
         WHERE id = $1::uuid AND used_at IS NULL",
    )
    .bind(&token_id)
    .execute(pool)
    .await?;

    let user_id: Uuid = user_id_str.parse().map_err(|_| sqlx::Error::ColumnDecode {
        index: "user_id".to_string(),
        source: Box::new(std::fmt::Error),
    })?;

    Ok(Some(user_id))
}

pub async fn has_pending_token(pool: &sqlx::AnyPool, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM password_reset_tokens \
         WHERE user_id = $1::uuid AND used_at IS NULL AND expires_at > NOW()",
    )
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await?;

    Ok(row.0 > 0)
}
