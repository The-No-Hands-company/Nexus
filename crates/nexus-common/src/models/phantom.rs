//! Phantom Protocol — Post-quantum identity and E2EE for Nexus users.
//!
//! Each user can optionally attach a PhantomIdentity to their account.
//! The Phantom identity is stored separately from the User model to avoid
//! modifying the existing User construction sites.

use base64::Engine;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A Phantom post-quantum identity attached to a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhantomIdentity {
    pub user_id: Uuid,
    pub did: String,
    pub kem_public: String,     // base64 Kyber-1024 public key
    pub signing_public: String, // base64 Dilithium-5 public key
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl PhantomIdentity {
    /// Generate a new Phantom identity for a user.
    pub fn generate(user_id: Uuid, username: &str) -> Self {
        use pqcrypto_kyber::kyber1024;
        use pqcrypto_dilithium::dilithium5;
        use pqcrypto_traits::kem::PublicKey as _;
        use pqcrypto_traits::sign::PublicKey as _;

        let (kem_pk, _) = kyber1024::keypair();
        let (sig_pk, _) = dilithium5::keypair();

        let did = format!(
            "did:phantom:{}",
            &hex::encode(blake3::hash(username.as_bytes()).as_bytes())[..16]
        );

        Self {
            user_id,
            did,
            kem_public: b64(&kem_pk.as_bytes()),
            signing_public: b64(&sig_pk.as_bytes()),
            created_at: chrono::Utc::now(),
        }
    }
}

/// Secret key material (never serialized, never sent to clients).
#[derive(Debug, Clone)]
pub struct PhantomSecretKeys {
    pub user_id: Uuid,
    pub kem_secret: Vec<u8>,
    pub signing_secret: Vec<u8>,
}

impl PhantomSecretKeys {
    /// Generate secret keys alongside the public identity.
    pub fn generate(user_id: Uuid) -> Self {
        use pqcrypto_kyber::kyber1024;
        use pqcrypto_dilithium::dilithium5;
        use pqcrypto_traits::kem::SecretKey as _;
        use pqcrypto_traits::sign::SecretKey as _;

        let (_, kem_sk) = kyber1024::keypair();
        let (_, sig_sk) = dilithium5::keypair();

        Self {
            user_id,
            kem_secret: kem_sk.as_bytes().to_vec(),
            signing_secret: sig_sk.as_bytes().to_vec(),
        }
    }
}

/// Result of Phantom identity generation (public + secret).
pub struct PhantomKeyPair {
    pub identity: PhantomIdentity,
    pub secrets: PhantomSecretKeys,
}

impl PhantomKeyPair {
    pub fn generate(user_id: Uuid, username: &str) -> Self {
        use pqcrypto_kyber::kyber1024;
        use pqcrypto_dilithium::dilithium5;
        use pqcrypto_traits::kem::{PublicKey as _, SecretKey as _};
        use pqcrypto_traits::sign::{PublicKey as _, SecretKey as _};

        let (kem_pk, kem_sk) = kyber1024::keypair();
        let (sig_pk, sig_sk) = dilithium5::keypair();

        let did = format!(
            "did:phantom:{}",
            &hex::encode(blake3::hash(username.as_bytes()).as_bytes())[..16]
        );

        let identity = PhantomIdentity {
            user_id,
            did,
            kem_public: b64(&kem_pk.as_bytes()),
            signing_public: b64(&sig_pk.as_bytes()),
            created_at: chrono::Utc::now(),
        };

        let secrets = PhantomSecretKeys {
            user_id,
            kem_secret: kem_sk.as_bytes().to_vec(),
            signing_secret: sig_sk.as_bytes().to_vec(),
        };

        Self { identity, secrets }
    }
}

/// Sign a message with Dilithium-5.
pub fn sign_message(secret_bytes: &[u8], message: &[u8]) -> Option<Vec<u8>> {
    use pqcrypto_dilithium::dilithium5;
    use pqcrypto_traits::sign::{SecretKey as _, DetachedSignature as _};

    let sk = dilithium5::SecretKey::from_bytes(secret_bytes).ok()?;
    let sig = dilithium5::detached_sign(message, &sk);
    Some(sig.as_bytes().to_vec())
}

/// Verify a Dilithium-5 signature.
pub fn verify_signature(public_b64: &str, message: &[u8], signature: &[u8]) -> bool {
    use pqcrypto_dilithium::dilithium5;
    use pqcrypto_traits::sign::{PublicKey as _, DetachedSignature};

    let pk_bytes = base64_decode(public_b64);
    let (Ok(pk), Ok(sig)) = (
        dilithium5::PublicKey::from_bytes(&pk_bytes),
        DetachedSignature::from_bytes(signature),
    ) else { return false };
    dilithium5::verify_detached_signature(&sig, message, &pk).is_ok()
}

fn b64(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn base64_decode(s: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD.decode(s).unwrap_or_default()
}

// Database access for Phantom identities deliberately does NOT live here.
// nexus-common is the foundation layer — "no business logic, just primitives and
// contracts" — and it sits below nexus-db, so it has no business issuing
// queries. The storage side is nexus_db::repository::phantom:
// `get_identity`, `get_secret_keys`, `insert_identity`, `delete_identity`.
//
// This module previously also carried sign_message_content() plus private
// phantom_identity()/phantom_secret_keys() helpers. They had never compiled
// (they referenced their own module through `crate::`, bound Uuid values to an
// AnyPool that has no Uuid encoder, and declared a row struct with no
// FromRow<AnyRow> impl) and had no callers anywhere in the workspace; their own
// doc comment pointed at nexus_db::repository::phantom for the real thing.
// Their INSERT was also unwritable as specified: it names message_id but the
// function took no message_id, supplying three binds for four placeholders.
// Signing-on-send belongs next to the other queries in nexus-db, built on
// get_secret_keys() and the pure sign_message() above.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_phantom_identity() {
        let user_id = Uuid::now_v7();
        let id = PhantomIdentity::generate(user_id, "alice");
        assert!(id.did.starts_with("did:phantom:"));
        assert!(!id.kem_public.is_empty());
        assert!(!id.signing_public.is_empty());
    }

    #[test]
    fn test_sign_verify() {
        let user_id = Uuid::now_v7();
        let keys = PhantomKeyPair::generate(user_id, "bob");
        let msg = b"Hello from Phantom E2EE in Nexus!";
        let sig = sign_message(&keys.secrets.signing_secret, msg).unwrap();
        assert!(verify_signature(&keys.identity.signing_public, msg, &sig));
        assert!(!verify_signature(&keys.identity.signing_public, b"tampered", &sig));
    }
}

