//! Email verification token repository.
//!
//! Verification tokens are one-time-use random strings.
//! Only the SHA-256 hash is stored — the raw token is sent via email and
//! immediately forgotten by the server (same pattern as password reset tokens).

use chrono::{DateTime, Duration, Utc};
use rand::{RngCore};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Number of hours a verification token remains valid.
const TOKEN_TTL_HOURS: i64 = 24;

/// Length of the randomly generated token in bytes (hex-encodes to 64 chars).
const TOKEN_BYTES: usize = 32;

/// Generate a cryptographically random email verification token.
///
/// Returns `(raw_token, token_hash, expires_at)`.  The raw token is sent to
/// the user; only the hash persists in the database.
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

/// Helper to hash a raw token for DB lookup.
fn hash_token(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

/// Persist a new email verification token, replacing any existing one for the user.
pub async fn upsert_token(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    // Remove previously issued tokens for this user
    sqlx::query("DELETE FROM email_verification_tokens WHERE user_id = $1::uuid")
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query(
        "INSERT INTO email_verification_tokens (id, user_id, token_hash, expires_at) \
         VALUES (gen_random_uuid(), $1::uuid, $2, $3)",
    )
    .bind(user_id.to_string())
    .bind(token_hash)
    .bind(expires_at.to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

/// Attempt to consume a verification token.
///
/// Returns the `user_id` if the token is valid and not expired, `None` otherwise.
pub async fn consume_token(
    pool: &sqlx::AnyPool,
    raw_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let hash = hash_token(raw_token);

    // Find and delete atomically in two steps (ANY driver doesn't support RETURNING on DELETE)
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT id::text, user_id::text FROM email_verification_tokens \
         WHERE token_hash = $1 AND expires_at > NOW()",
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await?;

    let Some((token_id, user_id_str)) = row else {
        return Ok(None);
    };

    // Consume the token
    sqlx::query("DELETE FROM email_verification_tokens WHERE id = $1::uuid")
        .bind(&token_id)
        .execute(pool)
        .await?;

    let user_id: Uuid = user_id_str
        .parse()
        .map_err(|_| sqlx::Error::ColumnDecode {
            index: "user_id".to_string(),
            source: Box::new(std::fmt::Error),
        })?;

    Ok(Some(user_id))
}

/// Check whether a pending (unexpired) token exists for the user.
pub async fn has_pending_token(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM email_verification_tokens \
         WHERE user_id = $1::uuid AND expires_at > NOW()",
    )
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(row.0 > 0)
}
