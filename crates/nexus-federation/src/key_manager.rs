//! DB-backed server signing key management.
//!
//! On startup, `KeyManager::load_or_generate` queries `federation_keys` for the
//! most recent non-expired active key. If none exists (first run or all keys
//! have expired), it generates a fresh Ed25519 pair, persists it, and returns it.
//!
//! # Key rotation
//! Keys are valid for 90 days (`KEY_TTL_DAYS`). To rotate: deactivate the old
//! row (`is_active = FALSE`) and restart — the manager will generate a new one.

use std::sync::Arc;

use anyhow::anyhow;
use chrono::{Duration, Utc};
use sqlx::{PgPool, Row as _};
use tracing::{info, warn};

use crate::{error::FederationError, keys::ServerKeyPair};

const KEY_TTL_DAYS: i64 = 90;

// ─── Key manager ─────────────────────────────────────────────────────────────

/// Handles loading or provisioning this server's Ed25519 signing key from
/// the `federation_keys` PostgreSQL table.
pub struct KeyManager {
    pool: PgPool,
}

impl KeyManager {
    /// Create a new `KeyManager` backed by the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Return the active, non-expired `ServerKeyPair` for this server.
    ///
    /// Steps:
    /// 1. Query `federation_keys` for the newest active non-expired key.
    /// 2. If found, reconstruct from stored seed and return it.
    /// 3. If not found, generate a new pair, persist it, and return it.
    pub async fn load_or_generate(&self) -> Result<Arc<ServerKeyPair>, FederationError> {
        // ── 1. Try loading from DB ────────────────────────────────────────────
        let row = sqlx::query(
            "SELECT key_id, seed_bytes \
             FROM federation_keys \
             WHERE is_active = TRUE AND expires_at > NOW() \
             ORDER BY created_at DESC \
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FederationError::Other(anyhow!(e)))?;

        if let Some(row) = row {
            let key_id: String = row
                .try_get("key_id")
                .map_err(|e| FederationError::Other(anyhow!(e)))?;
            let seed_bytes: Vec<u8> = row
                .try_get("seed_bytes")
                .map_err(|e| FederationError::Other(anyhow!(e)))?;

            let kp = ServerKeyPair::from_seed(&seed_bytes)?;
            info!("Federation: loaded active signing key {}", key_id);
            return Ok(Arc::new(kp));
        }

        // ── 2. Nothing found — generate + persist ────────────────────────────
        warn!("No active federation signing key — generating a new Ed25519 key pair");

        let kp = ServerKeyPair::generate();
        let expires_at = Utc::now() + Duration::days(KEY_TTL_DAYS);

        sqlx::query(
            "INSERT INTO federation_keys \
             (key_id, seed_bytes, public_key_b64, expires_at, is_active) \
             VALUES ($1, $2, $3, $4, TRUE) \
             ON CONFLICT (key_id) DO NOTHING",
        )
        .bind(&kp.key_id)
        .bind(kp.seed_bytes().to_vec())
        .bind(kp.public_key_base64())
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| FederationError::Other(anyhow!(e)))?;

        info!("Federation: generated and persisted new signing key {}", kp.key_id);
        Ok(Arc::new(kp))
    }
}
