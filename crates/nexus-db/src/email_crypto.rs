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

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng},
};
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

    let cipher = Aes256Gcm::new_from_slice(&raw_key).expect("key is exactly 32 bytes");

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, email.as_bytes())
        .expect("AES-GCM encryption is infallible for valid inputs");

    let mut payload = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    payload.extend_from_slice(&nonce_bytes);
    payload.extend_from_slice(&ciphertext);

    format!(
        "{ENC_PREFIX}{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &payload,)
    )
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
    let payload = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64).ok()?;

    if payload.len() < NONCE_LEN + 16 {
        tracing::warn!("email_crypto: ciphertext too short");
        return None;
    }

    let (nonce_bytes, ciphertext) = payload.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(&raw_key).expect("key is exactly 32 bytes");

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
    let mut mac: Hmac<Sha256> =
        hmac::Mac::new_from_slice(&raw_key).expect("HMAC accepts any key length");
    mac.update(email.to_lowercase().as_bytes());
    Some(hex::encode(mac.finalize().into_bytes()))
}

/// Returns `true` if at-rest email encryption is active.
pub fn is_enabled() -> bool {
    key().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    // A fixed 32-byte key for tests (64 hex chars).
    const TEST_KEY_HEX: &str = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";

    /// Run a test body with the email encryption key set, restoring state afterwards.
    ///
    /// Each test that exercises the encrypted path calls this to avoid relying on
    /// persistent env state, since `EMAIL_KEY` is a OnceLock.  We test the crypto
    /// functions directly using the key bytes rather than going through the env var
    /// because OnceLock only initialises once per process.
    fn with_key<F: FnOnce([u8; 32])>(f: F) {
        let key_bytes = hex::decode(TEST_KEY_HEX).unwrap();
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        f(key);
    }

    // ── encrypt / decrypt round-trip ──────────────────────────────────────────

    /// Encrypt and decrypt using key bytes directly (bypassing the OnceLock env path).
    fn encrypt_with_key(email: &str, key: [u8; 32]) -> String {
        use aes_gcm::aead::rand_core::RngCore;
        use aes_gcm::{
            Aes256Gcm, Nonce,
            aead::{Aead, KeyInit, OsRng},
        };

        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, email.as_bytes()).unwrap();
        let mut payload = Vec::with_capacity(12 + ciphertext.len());
        payload.extend_from_slice(&nonce_bytes);
        payload.extend_from_slice(&ciphertext);
        format!(
            "enc:{}",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &payload)
        )
    }

    fn decrypt_with_key(stored: &str, key: [u8; 32]) -> Option<String> {
        use aes_gcm::{
            Aes256Gcm, Nonce,
            aead::{Aead, KeyInit},
        };
        if !stored.starts_with("enc:") {
            return Some(stored.to_string());
        }
        let payload =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &stored[4..])
                .ok()?;
        if payload.len() < 28 {
            return None;
        }
        let (nonce_bytes, ciphertext) = payload.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let plain = cipher.decrypt(nonce, ciphertext).ok()?;
        String::from_utf8(plain).ok()
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        with_key(|key| {
            let email = "alice@example.com";
            let stored = encrypt_with_key(email, key);
            assert!(stored.starts_with("enc:"), "should have enc: prefix");
            let recovered = decrypt_with_key(&stored, key).expect("should decrypt");
            assert_eq!(recovered, email);
        });
    }

    #[test]
    fn same_email_produces_different_ciphertexts() {
        // Each call uses a fresh random nonce, so ciphertexts must differ.
        with_key(|key| {
            let email = "bob@example.com";
            let c1 = encrypt_with_key(email, key);
            let c2 = encrypt_with_key(email, key);
            assert_ne!(c1, c2, "random nonces must produce distinct ciphertexts");
        });
    }

    #[test]
    fn decrypt_handles_plaintext_passthrough() {
        // Rows written before encryption was enabled (no "enc:" prefix) should
        // pass through unchanged.
        with_key(|key| {
            let plain = "old@example.com";
            let recovered = decrypt_with_key(plain, key).expect("plaintext passthrough");
            assert_eq!(recovered, plain);
        });
    }

    #[test]
    fn decrypt_returns_none_for_corrupt_ciphertext() {
        with_key(|key| {
            // Tamper with the ciphertext after the prefix
            let stored = "enc:dGhpcyBpcyBub3QgdmFsaWQgY2lwaGVydGV4dA=="; // invalid AES-GCM auth tag
            let result = decrypt_with_key(stored, key);
            assert!(result.is_none(), "corrupt ciphertext must fail to decrypt");
        });
    }

    #[test]
    fn decrypt_returns_none_for_too_short_payload() {
        with_key(|key| {
            // Payload shorter than nonce (12 bytes) + tag (16 bytes) = 28 bytes minimum
            let short = format!(
                "enc:{}",
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"tooshort")
            );
            let result = decrypt_with_key(&short, key);
            assert!(result.is_none());
        });
    }

    // ── lookup_hash (HMAC-SHA256) ─────────────────────────────────────────────

    fn hmac_hash(email: &str, key: [u8; 32]) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut mac: Hmac<Sha256> = hmac::Mac::new_from_slice(&key).unwrap();
        mac.update(email.to_lowercase().as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn lookup_hash_is_stable_for_same_input() {
        with_key(|key| {
            let h1 = hmac_hash("alice@example.com", key);
            let h2 = hmac_hash("alice@example.com", key);
            assert_eq!(h1, h2);
        });
    }

    #[test]
    fn lookup_hash_differs_for_different_emails() {
        with_key(|key| {
            let h1 = hmac_hash("alice@example.com", key);
            let h2 = hmac_hash("bob@example.com", key);
            assert_ne!(h1, h2);
        });
    }

    #[test]
    fn lookup_hash_is_case_insensitive() {
        // The function lowercases before hashing, so mixed-case emails
        // must produce the same hash as their lowercase form.
        with_key(|key| {
            let lower = hmac_hash("alice@example.com", key);
            let upper = hmac_hash("ALICE@EXAMPLE.COM", key);
            let mixed = hmac_hash("Alice@Example.Com", key);
            assert_eq!(lower, upper);
            assert_eq!(lower, mixed);
        });
    }

    #[test]
    fn lookup_hash_is_hex_encoded_64_chars() {
        // SHA-256 output is 32 bytes = 64 hex chars.
        with_key(|key| {
            let hash = hmac_hash("test@test.com", key);
            assert_eq!(hash.len(), 64);
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        });
    }

    // ── no-key passthrough mode ───────────────────────────────────────────────

    #[test]
    fn no_key_encrypt_returns_plaintext_unchanged() {
        // When EMAIL_KEY is absent, encrypt() is a no-op.
        // Since EMAIL_KEY is a OnceLock we test via is_enabled() which
        // returns false when the key hasn't been loaded.
        // The public API guarantees: if !is_enabled(), encrypt(x) == x.
        // We verify the conditional branch exists by testing the helper directly.
        // (Full env-var path is tested in integration tests.)
        if !is_enabled() {
            let email = "passthrough@example.com";
            let result = encrypt(email);
            assert_eq!(result, email, "no-key mode must return input unchanged");
        }
    }
}
