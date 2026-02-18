//! E2EE crypto utilities — server-side helpers.
//!
//! The server deliberately does NOT perform Signal Protocol cryptography.
//! That happens exclusively on clients. This module contains only:
//!
//! - **Safety number computation** — a human-verifiable fingerprint of two
//!   identity keys that users compare out-of-band to detect MITM attacks.
//! - **Key material validation** — basic sanity checks on uploaded key blobs
//!   (correct base64 encoding, expected byte lengths for X25519 / Ed25519).
//! - **Utility helpers** shared across the API and repository layers.
//!
//! # Safety Number Algorithm
//! Inspired by Signal's safety number spec:
//! 1. Decode both Ed25519 identity keys from base64 → 32 bytes each.
//! 2. Sort the two (user_id_bytes || identity_key_bytes) pairs lexicographically.
//! 3. Concatenate sorted pairs.
//! 4. SHA-512 hash the result.
//! 5. Encode the first 30 bytes as 10 groups of 5 decimal digits (60 digits total).

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use sha2::{Digest, Sha512};
use uuid::Uuid;

/// Byte length of an Ed25519 public key.
const ED25519_PUBLIC_KEY_LEN: usize = 32;
/// Byte length of an X25519 public key.
const X25519_PUBLIC_KEY_LEN: usize = 32;

// ============================================================
// Validation
// ============================================================

/// Error returned when uploaded key material fails validation.
#[derive(Debug, thiserror::Error)]
pub enum KeyValidationError {
    #[error("Key is not valid base64: {0}")]
    NotBase64(String),
    #[error("Key has wrong length: expected {expected} bytes, got {actual}")]
    WrongLength { expected: usize, actual: usize },
    #[error("Signature is not valid base64: {0}")]
    BadSignature(String),
}

/// Validate that a string is valid base64 and decodes to exactly `expected_len` bytes.
pub fn validate_key_bytes(
    encoded: &str,
    expected_len: usize,
    label: &str,
) -> Result<Vec<u8>, KeyValidationError> {
    let bytes = B64
        .decode(encoded)
        .map_err(|_| KeyValidationError::NotBase64(label.to_owned()))?;
    if bytes.len() != expected_len {
        return Err(KeyValidationError::WrongLength {
            expected: expected_len,
            actual: bytes.len(),
        });
    }
    Ok(bytes)
}

/// Validate an Ed25519 public identity key (32 bytes, base64-encoded).
pub fn validate_identity_key(encoded: &str) -> Result<Vec<u8>, KeyValidationError> {
    validate_key_bytes(encoded, ED25519_PUBLIC_KEY_LEN, "identity_key")
}

/// Validate an X25519 public key (signed pre-key or one-time pre-key, 32 bytes).
pub fn validate_x25519_key(encoded: &str, label: &str) -> Result<Vec<u8>, KeyValidationError> {
    validate_key_bytes(encoded, X25519_PUBLIC_KEY_LEN, label)
}

/// Validate an Ed25519 signature (64 bytes, base64-encoded).
pub fn validate_signature(encoded: &str) -> Result<Vec<u8>, KeyValidationError> {
    let bytes = B64
        .decode(encoded)
        .map_err(|_| KeyValidationError::BadSignature("signed_pre_key_sig".to_owned()))?;
    if bytes.len() != 64 {
        return Err(KeyValidationError::WrongLength {
            expected: 64,
            actual: bytes.len(),
        });
    }
    Ok(bytes)
}

// ============================================================
// Safety Number
// ============================================================

/// Compute a safety number (60-digit decimal fingerprint) for a pair of users.
///
/// Both `identity_key_a` and `identity_key_b` must be base64-encoded Ed25519
/// public keys (32 bytes each after decoding).
///
/// Returns the 60-digit fingerprint string, or an error if either key is invalid.
pub fn compute_safety_number(
    user_id_a: Uuid,
    identity_key_a: &str,
    user_id_b: Uuid,
    identity_key_b: &str,
) -> Result<String, KeyValidationError> {
    let key_a = validate_identity_key(identity_key_a)?;
    let key_b = validate_identity_key(identity_key_b)?;

    // Build sortable (user_id_bytes || key_bytes) pairs
    let mut pair_a = user_id_a.as_bytes().to_vec();
    pair_a.extend_from_slice(&key_a);

    let mut pair_b = user_id_b.as_bytes().to_vec();
    pair_b.extend_from_slice(&key_b);

    // Sort deterministically so both sides produce the same number
    let (first, second) = if pair_a <= pair_b {
        (pair_a, pair_b)
    } else {
        (pair_b, pair_a)
    };

    // Hash
    let mut hasher = Sha512::new();
    hasher.update(&first);
    hasher.update(&second);
    let digest = hasher.finalize();

    // Encode first 30 bytes as 10 groups of 5 decimal digits
    let fingerprint = digest[..30]
        .chunks(3)
        .map(|chunk| {
            let n = (chunk[0] as u32) << 16 | (chunk[1] as u32) << 8 | (chunk[2] as u32);
            format!("{:05}", n % 100_000)
        })
        .collect::<Vec<_>>()
        .join(" ");

    Ok(fingerprint)
}

// ============================================================
// Helpers
// ============================================================

/// Encode arbitrary bytes to base64 (standard alphabet, padded).
pub fn to_base64(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

/// Decode base64 to bytes, returning `None` on failure.
pub fn from_base64(encoded: &str) -> Option<Vec<u8>> {
    B64.decode(encoded).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_number_is_deterministic() {
        // Two 32-byte all-zero keys as base64
        let key_a = to_base64(&[0u8; 32]);
        let key_b = to_base64(&[1u8; 32]);
        let uid_a = Uuid::nil();
        let uid_b = Uuid::max();

        let sn1 = compute_safety_number(uid_a, &key_a, uid_b, &key_b).unwrap();
        let sn2 = compute_safety_number(uid_b, &key_b, uid_a, &key_a).unwrap();
        assert_eq!(sn1, sn2, "Safety number must be symmetric");
        assert_eq!(sn1.replace(' ', "").len(), 50, "Should be 10 × 5 digits");
    }

    #[test]
    fn validate_identity_key_bad_length() {
        let short = to_base64(&[0u8; 16]);
        assert!(validate_identity_key(&short).is_err());
    }
}
