//! Transparent email encryption at rest.
//!
//! When `NEXUS__SECURITY__EMAIL_KEY` is set to a 64-character hex string
//! (32 bytes), emails are encrypted with AES-256-GCM before being written to
//! the database and decrypted on read.  A keyed HMAC-SHA256 digest is stored
//! in the separate `email_hash` column so that lookup-by-email still works
//! without decrypting every row.
//!
//! Ciphertext format (stored in the `email` column):
//!   `enc:<base64(12-byte nonce || ciphertext)>`
//!
//! When the env var is absent the module is a no-op: plaintext is written and
//! read without modification.  This lets existing deployments opt in by simply
//! setting the variable and running a one-time migration.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use aes_gcm::aead::rand_core::RngCore;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::OnceLock;

const ENC_PREFIX: &str = "enc:";
const NONCE_LEN: usize = 12;

// ── Key loading ──────────────────────────────────────────────────────────────

static EMAIL_KEY: OnceLock<Option<[u8; 32]>> = OnceLock::new();

fn load_key() -> Option<[u8; 32]> {
    let hex_key = std::env::var("NEXUS__SECURITY__EMAIL_KEY").ok()?;
    let bytes = hex::decode(hex_key.trim()).ok()?;
    if bytes.len() != 32 {
        tracing::warn!(
            "NEXUS__SECURITY__EMAIL_KEY must be 64 hex chars (32 bytes); email encryption disabled"
        );
        return None;
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Some(key)
}

fn key() -> Option<[u8; 32]> {
    *EMAIL_KEY.get_or_init(load_key)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Encrypt an email address for storage.
///
/// If no key is configured returns the email unchanged (plaintext).
pub fn encrypt(email: &str) -> String {
    let Some(raw_key) = key() else {
        return email.to_string();
    };

    let cipher = Aes256Gcm::new_from_slice(&raw_key)
        .expect("key is exactly 32 bytes");

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, email.as_bytes())
        .expect("AES-GCM encryption is infallible for valid inputs");

    let mut payload = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    payload.extend_from_slice(&nonce_bytes);
    payload.extend_from_slice(&ciphertext);

    format!("{ENC_PREFIX}{}", base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &payload,
    ))
}

/// Decrypt a stored email value.
///
/// - Values prefixed with `"enc:"` are decrypted with AES-256-GCM.
/// - Plain values are returned as-is (backward-compat / no-key mode).
/// - Returns `None` if decryption fails (bad key or corrupt data).
pub fn decrypt(stored: &str) -> Option<String> {
    let Some(raw_key) = key() else {
        return Some(stored.to_string());
    };

    if !stored.starts_with(ENC_PREFIX) {
        // Plaintext row written before encryption was enabled — return as-is.
        return Some(stored.to_string());
    }

    let b64 = &stored[ENC_PREFIX.len()..];
    let payload = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        b64,
    ).ok()?;

    if payload.len() < NONCE_LEN + 16 {
        tracing::warn!("email_crypto: ciphertext too short");
        return None;
    }

    let (nonce_bytes, ciphertext) = payload.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(&raw_key)
        .expect("key is exactly 32 bytes");

    let plaintext = cipher.decrypt(nonce, ciphertext).ok()?;
    String::from_utf8(plaintext).ok()
}

/// Compute a stable HMAC-SHA256 digest over the lowercased email.
///
/// Used as the lookup key in the `email_hash` column so we can find a user
/// by email without scanning and decrypting the entire table.
///
/// Returns `None` when no encryption key is configured (legacy mode — email
/// lookups go directly against the plaintext `email` column).
pub fn lookup_hash(email: &str) -> Option<String> {
    let raw_key = key()?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&raw_key)
        .expect("HMAC accepts any key length");
    mac.update(email.to_lowercase().as_bytes());
    Some(hex::encode(mac.finalize().into_bytes()))
}

/// Returns `true` if at-rest email encryption is active.
pub fn is_enabled() -> bool {
    key().is_some()
}
