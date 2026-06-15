//! Phantom Identity repository — PQ key storage for Nexus users.
//!
//! Stores post-quantum identity keys (Kyber-1024 + Dilithium-5).
//! Secret keys are stored as raw bytes — encrypt before storage in production.

use nexus_common::models::phantom::{PhantomIdentity, PhantomSecretKeys};
use sqlx::AnyPool;
use uuid::Uuid;

/// Create the phantom_identities table (idempotent migration).
pub async fn run_migration(pool: &AnyPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS phantom_identities (
            user_id      UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
            did          TEXT NOT NULL UNIQUE,
            kem_public   TEXT NOT NULL,
            kem_secret   BYTEA,
            signing_public  TEXT NOT NULL,
            signing_secret  BYTEA,
            created_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Insert a new Phantom identity for a user.
pub async fn insert_identity(
    pool: &AnyPool,
    identity: &PhantomIdentity,
    secrets: &PhantomSecretKeys,
) -> Result<PhantomIdentity, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO phantom_identities (user_id, did, kem_public, kem_secret, signing_public, signing_secret, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (user_id) DO NOTHING
         RETURNING user_id, did, kem_public, signing_public, created_at"
    )
    .bind(identity.user_id)
    .bind(&identity.did)
    .bind(&identity.kem_public)
    .bind(&secrets.kem_secret)
    .bind(&identity.signing_public)
    .bind(&secrets.signing_secret)
    .bind(identity.created_at)
    .fetch_one(pool)
    .await
}

/// Get a user's Phantom identity (public fields only).
pub async fn get_identity(pool: &AnyPool, user_id: Uuid) -> Result<Option<PhantomIdentity>, sqlx::Error> {
    sqlx::query_as(
        "SELECT user_id, did, kem_public, signing_public, created_at
         FROM phantom_identities WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// Get a user's Phantom secret keys (for server-side signing).
pub async fn get_secret_keys(pool: &AnyPool, user_id: Uuid) -> Result<Option<PhantomSecretKeys>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct SecretRow {
        user_id: Uuid,
        kem_secret: Option<Vec<u8>>,
        signing_secret: Option<Vec<u8>>,
    }

    let row: Option<SecretRow> = sqlx::query_as(
        "SELECT user_id, kem_secret, signing_secret FROM phantom_identities WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|r| {
        Some(PhantomSecretKeys {
            user_id: r.user_id,
            kem_secret: r.kem_secret?,
            signing_secret: r.signing_secret?,
        })
    }))
}

/// Delete a user's Phantom identity.
pub async fn delete_identity(pool: &AnyPool, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let rows = sqlx::query("DELETE FROM phantom_identities WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}
