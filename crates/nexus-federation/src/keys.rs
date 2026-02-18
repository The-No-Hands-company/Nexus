//! Ed25519 server signing key management.
//!
//! Each Nexus server possesses an Ed25519 key pair used to sign outbound
//! federation requests and events. Remote servers verify these signatures using
//! the public key fetched from `/_nexus/key/v2/server`.
//!
//! # Key IDs
//! Key IDs follow the Matrix convention: `ed25519:<fingerprint>`.
//! The fingerprint is the first 5 bytes of the public key, hex-encoded (10 chars).
//! Example: `ed25519:3f9a2c`.
//!
//! # Storage
//! Keys are stored in the `federation_keys` PostgreSQL table. On startup, the
//! server loads the most recent non-expired key. If none exists, a new key pair
//! is generated and persisted.

use base64::Engine as _;
use chrono::{Duration, Utc};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};

use crate::error::FederationError;

// How long a key is considered valid before rotation (90 days).
const KEY_TTL_DAYS: i64 = 90;

// ─── Key pair ────────────────────────────────────────────────────────────────

/// An Ed25519 signing key pair for this Nexus server.
///
/// The `ServerKeyPair` is the single source of truth for all outbound
/// federation signatures.
pub struct ServerKeyPair {
    /// Key ID in the format `ed25519:<10-char-hex>`.
    pub key_id: String,
    signing_key: SigningKey,
}

impl ServerKeyPair {
    /// Generate a brand-new random Ed25519 key pair.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let key_id = derive_key_id(signing_key.verifying_key().as_bytes());
        Self { key_id, signing_key }
    }

    /// Reconstruct a `ServerKeyPair` from raw 32-byte seed bytes (as stored in DB).
    pub fn from_seed(seed: &[u8]) -> Result<Self, FederationError> {
        let bytes: [u8; 32] = seed
            .try_into()
            .map_err(|_| FederationError::KeyLoad("seed must be exactly 32 bytes".into()))?;
        let signing_key = SigningKey::from_bytes(&bytes);
        let key_id = derive_key_id(signing_key.verifying_key().as_bytes());
        Ok(Self { key_id, signing_key })
    }

    /// Return the 32-byte seed (private key scalar) for persistence.
    pub fn seed_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Return the public verifying key.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Return the public key as a base64url-encoded string (for `/_nexus/key/v2/server`).
    pub fn public_key_base64(&self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(self.signing_key.verifying_key().as_bytes())
    }

    /// Sign arbitrary bytes and return the base64url-encoded signature.
    pub fn sign_bytes(&self, bytes: &[u8]) -> String {
        let sig = self.signing_key.sign(bytes);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.to_bytes())
    }

    /// Sign a canonical JSON string and return the base64url-encoded signature.
    pub fn sign_json(&self, canonical_json: &str) -> String {
        self.sign_bytes(canonical_json.as_bytes())
    }

    /// Build the `ServerKeyDocument` suitable for `/_nexus/key/v2/server`.
    pub fn to_key_document(&self, server_name: &str) -> ServerKeyDocument {
        use std::collections::HashMap;
        let mut keys = HashMap::new();
        keys.insert(
            self.key_id.clone(),
            crate::types::VerifyKey { key: self.public_key_base64() },
        );
        ServerKeyDocument {
            server_name: server_name.to_owned(),
            verify_keys: keys,
            valid_until_ts: (Utc::now() + Duration::days(KEY_TTL_DAYS)).timestamp_millis(),
        }
    }
}

// ─── Key document (wire format) ───────────────────────────────────────────────

/// The signed key document served at `/_nexus/key/v2/server`.
///
/// Remote servers cache this document to verify future request signatures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerKeyDocument {
    pub server_name: String,
    pub verify_keys: std::collections::HashMap<String, crate::types::VerifyKey>,
    /// Unix millisecond timestamp after which this document should be re-fetched.
    pub valid_until_ts: i64,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Derive a stable key ID from raw public key bytes.
///
/// Uses the first 6 bytes of the pubkey as a short hex fingerprint.
fn derive_key_id(pubkey_bytes: &[u8]) -> String {
    let fingerprint = hex::encode(&pubkey_bytes[..6]);
    format!("ed25519:{}", fingerprint)
}

/// Verify an Ed25519 signature.
///
/// * `pubkey_base64` — base64url-encoded 32-byte verifying key
/// * `sig_base64`    — base64url-encoded 64-byte signature
/// * `message`       — original signed bytes
pub fn verify_signature(
    pubkey_base64: &str,
    sig_base64: &str,
    message: &[u8],
) -> Result<(), FederationError> {
    use ed25519_dalek::Verifier;

    let pubkey_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(pubkey_base64)
        .map_err(|_| FederationError::InvalidSignature)?;

    let sig_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(sig_base64)
        .map_err(|_| FederationError::InvalidSignature)?;

    let verifying_key = VerifyingKey::from_bytes(
        pubkey_bytes
            .as_slice()
            .try_into()
            .map_err(|_| FederationError::InvalidSignature)?,
    )
    .map_err(|_| FederationError::InvalidSignature)?;

    let signature = ed25519_dalek::Signature::from_bytes(
        sig_bytes
            .as_slice()
            .try_into()
            .map_err(|_| FederationError::InvalidSignature)?,
    );

    verifying_key.verify(message, &signature).map_err(|_| FederationError::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_sign_verify() {
        let kp = ServerKeyPair::generate();
        let msg = b"hello nexus federation";
        let sig = kp.sign_bytes(msg);
        verify_signature(&kp.public_key_base64(), &sig, msg).expect("signature should verify");
    }

    #[test]
    fn from_seed_is_stable() {
        let kp1 = ServerKeyPair::generate();
        let seed = kp1.seed_bytes();
        let kp2 = ServerKeyPair::from_seed(&seed).unwrap();
        assert_eq!(kp1.key_id, kp2.key_id);
        assert_eq!(kp1.public_key_base64(), kp2.public_key_base64());
    }
}
