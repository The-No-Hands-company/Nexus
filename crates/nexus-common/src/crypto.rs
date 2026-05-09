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
//! 2. Sort the two (`user_id_bytes` || `identity_key_bytes`) pairs lexicographically.
//! 3. Concatenate sorted pairs.
//! 4. SHA-512 hash the result.
//! 5. Encode the first 30 bytes as 10 groups of 5 decimal digits (60 digits total).

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
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
///
/// # Errors
/// Returns `KeyValidationError` when decoding fails or the decoded length mismatches.
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
///
/// # Errors
/// Returns `KeyValidationError` when the key is not valid base64 or has an invalid length.
pub fn validate_identity_key(encoded: &str) -> Result<Vec<u8>, KeyValidationError> {
    validate_key_bytes(encoded, ED25519_PUBLIC_KEY_LEN, "identity_key")
}

/// Validate an X25519 public key (signed pre-key or one-time pre-key, 32 bytes).
///
/// # Errors
/// Returns `KeyValidationError` when the key is not valid base64 or has an invalid length.
pub fn validate_x25519_key(encoded: &str, label: &str) -> Result<Vec<u8>, KeyValidationError> {
    validate_key_bytes(encoded, X25519_PUBLIC_KEY_LEN, label)
}

/// Validate an Ed25519 signature (64 bytes, base64-encoded).
///
/// # Errors
/// Returns `KeyValidationError` when decoding fails or the signature length is invalid.
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
///
/// # Errors
/// Returns `KeyValidationError` when either identity key is invalid.
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
            let n = (u32::from(chunk[0])) << 16
                | (u32::from(chunk[1])) << 8
                | u32::from(chunk[2]);
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
#[must_use]
pub fn to_base64(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

/// Decode base64 to bytes, returning `None` on failure.
#[must_use]
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

// (appended to existing #[cfg(test)] mod tests block — compile error guard
// catches any accidental duplication)

#[test]
fn validate_identity_key_correct_length_succeeds() {
    let good = to_base64(&[0xabu8; 32]);
    assert!(validate_identity_key(&good).is_ok());
}

#[test]
fn validate_identity_key_not_base64_errors() {
    assert!(validate_identity_key("not!!base64!!").is_err());
}

#[test]
fn validate_identity_key_too_long_errors() {
    let long = to_base64(&[0u8; 64]);
    let err = validate_identity_key(&long).unwrap_err();
    assert!(matches!(
        err,
        KeyValidationError::WrongLength {
            expected: 32,
            actual: 64
        }
    ));
}

#[test]
fn validate_x25519_key_correct_length_succeeds() {
    let good = to_base64(&[0xcdu8; 32]);
    assert!(validate_x25519_key(&good, "spk").is_ok());
}

#[test]
fn validate_signature_64_bytes_succeeds() {
    let good = to_base64(&[0u8; 64]);
    assert!(validate_signature(&good).is_ok());
}

#[test]
fn validate_signature_wrong_length_errors() {
    let short = to_base64(&[0u8; 32]);
    assert!(validate_signature(&short).is_err());
}

#[test]
fn safety_number_different_users_produce_different_numbers() {
    let key = to_base64(&[1u8; 32]);
    let uid_a = Uuid::new_v4();
    let uid_b = Uuid::new_v4();
    let uid_c = Uuid::new_v4();

    let sn1 = compute_safety_number(uid_a, &key, uid_b, &key).unwrap();
    let sn2 = compute_safety_number(uid_a, &key, uid_c, &key).unwrap();
    assert_ne!(
        sn1, sn2,
        "different user pairs must produce different safety numbers"
    );
}

#[test]
fn safety_number_format_is_ten_groups_of_five_digits() {
    let key_a = to_base64(&[2u8; 32]);
    let key_b = to_base64(&[3u8; 32]);
    let sn = compute_safety_number(Uuid::nil(), &key_a, Uuid::max(), &key_b).unwrap();

    let groups: Vec<&str> = sn.split(' ').collect();
    assert_eq!(groups.len(), 10, "expected 10 space-separated groups");
    for g in &groups {
        assert_eq!(g.len(), 5, "each group must be exactly 5 chars: {g}");
        assert!(
            g.chars().all(|c| c.is_ascii_digit()),
            "group must be digits: {g}"
        );
    }
}

#[test]
fn safety_number_invalid_key_returns_error() {
    let good = to_base64(&[0u8; 32]);
    let bad = "not-valid-base64!!";
    assert!(compute_safety_number(Uuid::nil(), bad, Uuid::max(), &good).is_err());
    assert!(compute_safety_number(Uuid::nil(), &good, Uuid::max(), bad).is_err());
}

#[test]
fn from_base64_round_trips_arbitrary_bytes() {
    let original: Vec<u8> = (0u8..=255).collect();
    let encoded = to_base64(&original);
    let decoded = from_base64(&encoded).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn from_base64_returns_none_for_invalid_input() {
    assert!(from_base64("!!!invalid!!!").is_none());
}

#[test]
fn to_base64_empty_bytes_gives_empty_string() {
    assert_eq!(to_base64(&[]), "");
}
