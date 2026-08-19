//! E2EE crypto utilities — server-side helpers.
//!
//! Session cryptography — the Double Ratchet, message encryption, key
//! agreement — happens exclusively on clients. The server never holds a
//! private key and cannot read a message.
//!
//! It does, however, verify the one thing that binds a pre-key to an identity.
//! This module contains:
//!
//! - **Safety number computation** — a human-verifiable fingerprint of two
//!   identity keys that users compare out-of-band to detect MITM attacks.
//! - **Key material validation** — base64 and length checks on uploaded blobs.
//! - **Signed pre-key verification** — the Ed25519 signature over the pre-key,
//!   checked against the identity key. See below for why this is not optional.
//! - **Utility helpers** shared across the API and repository layers.
//!
//! # Why the server verifies the signed pre-key
//!
//! In X3DH the signed pre-key is the only element a peer cannot authenticate
//! for itself at first contact: it trusts that the pre-key it fetched really
//! belongs to the identity it is talking to, and that trust rests entirely on
//! the signature. This server previously checked only that the signature
//! decoded from base64 and was 64 bytes long, so 64 random bytes were accepted
//! and served on to peers as if authentic.
//!
//! Verifying it here does not make the server trusted — a client should still
//! verify for itself, and safety numbers exist for exactly that reason. It
//! makes the server *honest*: it refuses to distribute a bundle it can already
//! prove is unauthorised, instead of passing the problem to every peer.
//!
//! # Why dalek rather than a libsignal binding
//!
//! Signal does not publish libsignal to crates.io. The `libsignal-*` crates
//! there are third-party reimplementations or bindings to the deprecated
//! `libsignal-protocol-c`, and putting unaudited crypto in the one place that
//! authenticates keys would be worse than the gap it closes. The signature is
//! plain Ed25519 over the pre-key bytes — the identity key is Ed25519, not
//! X25519, so XEdDSA is not involved — and `ed25519-dalek` verifies exactly
//! that, is already vendored, and is widely audited.
//!
//! # Safety Number Algorithm
//! Inspired by Signal's safety number spec:
//! 1. Decode both Ed25519 identity keys from base64 → 32 bytes each.
//! 2. Sort the two (`user_id_bytes` || `identity_key_bytes`) pairs lexicographically.
//! 3. Concatenate sorted pairs.
//! 4. SHA-512 hash the result.
//! 5. Encode the first 30 bytes as 10 groups of 5 decimal digits (60 digits total).

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
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
    #[error("Identity key is not a valid Ed25519 public key: {0}")]
    BadIdentityKey(String),
    #[error("Signature does not verify against the identity key")]
    SignatureMismatch,
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

/// Verify that `signed_pre_key` really was signed by the holder of
/// `identity_key`.
///
/// All three arguments are base64. Returns the decoded signed pre-key on
/// success, so callers can persist the bytes they just authenticated rather
/// than decoding a second time and risking a mismatch.
///
/// # Errors
/// - `NotBase64` / `WrongLength` if any input is malformed.
/// - `BadIdentityKey` if the identity key is not a valid Ed25519 point.
///   Length alone does not establish this: a 32-byte value can still fail to
///   decompress to a curve point.
/// - `SignatureMismatch` if the signature does not verify. This is the case
///   that matters — it means the pre-key was not authorised by this identity.
pub fn verify_signed_pre_key(
    identity_key: &str,
    signed_pre_key: &str,
    signature: &str,
) -> Result<Vec<u8>, KeyValidationError> {
    let identity_bytes = validate_identity_key(identity_key)?;
    let pre_key_bytes = validate_x25519_key(signed_pre_key, "signed_pre_key")?;
    let sig_bytes = validate_signature(signature)?;

    // Lengths are already checked above, so these conversions cannot fail;
    // expect() would still be a panic path on a public endpoint, so they are
    // handled as errors.
    let identity_array: [u8; ED25519_PUBLIC_KEY_LEN] =
        identity_bytes
            .as_slice()
            .try_into()
            .map_err(|_| KeyValidationError::WrongLength {
                expected: ED25519_PUBLIC_KEY_LEN,
                actual: identity_bytes.len(),
            })?;
    let sig_array: [u8; 64] =
        sig_bytes
            .as_slice()
            .try_into()
            .map_err(|_| KeyValidationError::WrongLength {
                expected: 64,
                actual: sig_bytes.len(),
            })?;

    let verifying_key = VerifyingKey::from_bytes(&identity_array)
        .map_err(|_| KeyValidationError::BadIdentityKey("identity_key".to_owned()))?;
    let signature = Signature::from_bytes(&sig_array);

    verifying_key
        .verify(&pre_key_bytes, &signature)
        .map_err(|_| KeyValidationError::SignatureMismatch)?;

    Ok(pre_key_bytes)
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

    // ── signed pre-key verification ───────────────────────────────────────────
    //
    // Real Ed25519 keypairs, generated per test. A verifier proven only against
    // hand-written constants is a verifier nobody has watched reject a forgery.

    use ed25519_dalek::{Signer, SigningKey};
    use rand_core::{OsRng, RngCore};

    /// (identity_key_b64, signed_pre_key_b64, signature_b64)
    fn signed_bundle() -> (String, String, String) {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing = SigningKey::from_bytes(&seed);

        // A real X25519 public key is 32 bytes; any 32 bytes are structurally
        // valid here, and what is under test is the signature over them.
        let mut pre_key = [0u8; 32];
        OsRng.fill_bytes(&mut pre_key);

        let sig = signing.sign(&pre_key);
        (
            to_base64(&signing.verifying_key().to_bytes()),
            to_base64(&pre_key),
            to_base64(&sig.to_bytes()),
        )
    }

    #[test]
    fn a_correctly_signed_pre_key_verifies() {
        let (ik, spk, sig) = signed_bundle();
        let out = verify_signed_pre_key(&ik, &spk, &sig).expect("valid bundle must verify");
        assert_eq!(
            out,
            from_base64(&spk).unwrap(),
            "returns the bytes it authenticated"
        );
    }

    #[test]
    fn random_bytes_of_the_right_length_are_rejected() {
        // The exact hole this closes: the old check accepted any 64 bytes.
        let (ik, spk, _) = signed_bundle();
        let forged = to_base64(&[7u8; 64]);
        assert!(matches!(
            verify_signed_pre_key(&ik, &spk, &forged),
            Err(KeyValidationError::SignatureMismatch)
        ));
    }

    #[test]
    fn a_signature_from_a_different_identity_is_rejected() {
        // The attack that matters: a genuine signature made by the wrong key.
        // Serving that bundle lets an attacker substitute their own pre-key.
        let (ik_a, _, _) = signed_bundle();
        let (_, spk_b, sig_b) = signed_bundle();
        assert!(matches!(
            verify_signed_pre_key(&ik_a, &spk_b, &sig_b),
            Err(KeyValidationError::SignatureMismatch)
        ));
    }

    #[test]
    fn a_tampered_pre_key_is_rejected() {
        // Signature genuine, but the pre-key it covers was swapped underneath.
        let (ik, _, sig) = signed_bundle();
        let mut other = [0u8; 32];
        OsRng.fill_bytes(&mut other);
        assert!(matches!(
            verify_signed_pre_key(&ik, &to_base64(&other), &sig),
            Err(KeyValidationError::SignatureMismatch)
        ));
    }

    #[test]
    fn a_malformed_identity_key_is_rejected_not_panicked() {
        // 32 bytes that are not a meaningful Ed25519 public key. Length checks
        // pass, so this reaches the crypto — and on a public endpoint it must
        // come back as an error, never a panic.
        //
        // The variant is deliberately not pinned. dalek does not reject every
        // malformed encoding at `from_bytes`; some only fail when a signature
        // is checked against them, surfacing as SignatureMismatch rather than
        // BadIdentityKey. Which of the two comes back is dalek's business.
        // What this asserts is what callers depend on: rejected, and still
        // running.
        let (_, spk, sig) = signed_bundle();
        for bad in [[0xFFu8; 32], [0x00u8; 32], [0x01u8; 32]] {
            let r = verify_signed_pre_key(&to_base64(&bad), &spk, &sig);
            assert!(r.is_err(), "malformed identity key must not verify");
        }
    }


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
