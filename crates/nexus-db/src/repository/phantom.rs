//! Phantom Identity repository — PQ key storage for Nexus users.
//!
//! Stores post-quantum identity keys (Kyber-1024 + Dilithium-5).
//! Secret keys are stored as raw bytes — encrypt before storage in production.
//!
//! Everything here follows the AnyPool conventions used by the rest of this
//! layer: UUIDs and timestamps are bound as strings and cast in SQL (`$n::uuid`),
//! and read back through `select_cols` lists that cast them to `::text`, because
//! the Any driver cannot decode Postgres-native UUID/TIMESTAMPTZ/BYTEA.
//!
//! Secret key material is held as base64 TEXT for the same reason — the Any
//! driver has no BYTEA codec. The public keys were already stored that way, so
//! the whole row is now consistently text. `PhantomSecretKeys` keeps raw
//! `Vec<u8>` and the encoding stays an entirely storage-side concern.

use base64::Engine;
use nexus_common::models::phantom::{PhantomIdentity, PhantomSecretKeys};
use sqlx::{AnyPool, Row};
use uuid::Uuid;

use crate::select_cols::PHANTOM_IDENTITY_COLS;

fn b64_encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::STANDARD.decode(s).ok()
}

// The tables live in migrations/20260810000001_phantom_identities.sql, with the
// rest of the schema. This module used to carry its own run_migration() that
// created them — a second, divergent definition that nothing ever called, so the
// tables never existed and the mounted endpoints failed on every request.

/// Insert a new Phantom identity for a user.
///
/// `ON CONFLICT DO NOTHING` means a user who already has an identity yields no
/// row, so this returns `Ok(None)` rather than failing — callers decide whether
/// that is an error. (`fetch_one` used to turn it into `RowNotFound`.)
pub async fn insert_identity(
    pool: &AnyPool,
    identity: &PhantomIdentity,
    secrets: &PhantomSecretKeys,
) -> Result<Option<PhantomIdentity>, sqlx::Error> {
    let q = format!(
        "INSERT INTO phantom_identities \
             (user_id, did, kem_public, kem_secret, signing_public, signing_secret, created_at) \
         VALUES ($1::uuid, $2, $3, $4, $5, $6, $7::timestamptz) \
         ON CONFLICT (user_id) DO NOTHING \
         RETURNING {PHANTOM_IDENTITY_COLS}"
    );
    sqlx::query_as::<_, PhantomIdentity>(&q)
        .bind(identity.user_id.to_string())
        .bind(&identity.did)
        .bind(&identity.kem_public)
        .bind(b64_encode(&secrets.kem_secret))
        .bind(&identity.signing_public)
        .bind(b64_encode(&secrets.signing_secret))
        .bind(identity.created_at.to_rfc3339())
        .fetch_optional(pool)
        .await
}

/// Get a user's Phantom identity (public fields only).
pub async fn get_identity(pool: &AnyPool, user_id: Uuid) -> Result<Option<PhantomIdentity>, sqlx::Error> {
    let q = format!(
        "SELECT {PHANTOM_IDENTITY_COLS} FROM phantom_identities WHERE user_id = $1::uuid"
    );
    sqlx::query_as::<_, PhantomIdentity>(&q)
        .bind(user_id.to_string())
        .fetch_optional(pool)
        .await
}

/// Get a user's Phantom secret keys (for server-side signing).
///
/// Returns `None` when the user has no identity *or* when either secret is
/// absent — a half-populated row cannot sign or decrypt, so it is not a usable
/// key pair.
pub async fn get_secret_keys(pool: &AnyPool, user_id: Uuid) -> Result<Option<PhantomSecretKeys>, sqlx::Error> {
    // Read column-wise rather than through a derived FromRow: the Any driver
    // needs the uuid cast to text, and the secrets need base64-decoding.
    let row = sqlx::query(
        "SELECT user_id::text AS user_id, kem_secret, signing_secret \
         FROM phantom_identities WHERE user_id = $1::uuid"
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else { return Ok(None) };

    let id_str: String = row.try_get("user_id")?;
    let user_id = Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::Decode(Box::new(e) as _))?;
    let kem_secret: Option<String> = row.try_get("kem_secret")?;
    let signing_secret: Option<String> = row.try_get("signing_secret")?;

    Ok(match (kem_secret.as_deref().and_then(b64_decode), signing_secret.as_deref().and_then(b64_decode)) {
        (Some(kem_secret), Some(signing_secret)) => Some(PhantomSecretKeys {
            user_id,
            kem_secret,
            signing_secret,
        }),
        _ => None,
    })
}

/// Delete a user's Phantom identity.
pub async fn delete_identity(pool: &AnyPool, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let rows = sqlx::query("DELETE FROM phantom_identities WHERE user_id = $1::uuid")
        .bind(user_id.to_string())
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}
