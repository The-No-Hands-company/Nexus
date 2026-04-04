//! Authentication — JWT-based, privacy-first.
//!
//! No phone numbers. No government ID. No facial recognition. No age estimation.
//! Just a username, optional email, and a strong password.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
#[cfg(debug_assertions)]
use argon2::{Algorithm, Params, Version};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use uuid::Uuid;

// Re-export Claims and validate_token from nexus-common so existing code keeps working
pub use nexus_common::auth::{validate_token, Claims};

/// Token pair returned on login/register.
#[derive(Debug, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    /// Session UUID — used to persist the session row in `refresh_tokens`.
    /// Excluded from the JSON response; the client sees only the opaque tokens.
    #[serde(skip)]
    pub session_id: Uuid,
}

/// Returns the Argon2 instance appropriate for the current build profile.
fn argon2_instance() -> Argon2<'static> {
    #[cfg(debug_assertions)]
    {
        let params = Params::new(8 * 1024, 1, 1, None).expect("valid argon2 debug params");
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
    }
    #[cfg(not(debug_assertions))]
    {
        Argon2::default()
    }
}

/// Hash a password using Argon2id.
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2_instance().hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Verify a password against an Argon2id hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(argon2_instance()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Generate a JWT access token.
///
/// `session_id` becomes the `jti` claim — it identifies the owning session row
/// in `refresh_tokens` so individual sessions can be revoked.
pub fn generate_access_token(
    user_id: Uuid,
    username: &str,
    secret: &str,
    ttl_secs: u64,
    session_id: Uuid,
    two_fa_verified: bool,
    email_verified: bool,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        iat: now.timestamp(),
        exp: (now + Duration::seconds(ttl_secs as i64)).timestamp(),
        token_type: "access".to_string(),
        jti: session_id.to_string(),
        two_fa_verified,
        email_verified,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
}

/// Generate a JWT refresh token (longer-lived, same session JTI as the access token).
pub fn generate_refresh_token(
    user_id: Uuid,
    username: &str,
    secret: &str,
    ttl_secs: u64,
    session_id: Uuid,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        iat: now.timestamp(),
        exp: (now + Duration::seconds(ttl_secs as i64)).timestamp(),
        token_type: "refresh".to_string(),
        jti: session_id.to_string(),
        two_fa_verified: false,
        email_verified: false,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
}

/// Generate both access and refresh tokens for a newly authenticated session.
///
/// A fresh `session_id` (UUID v4) is minted here and embedded in both tokens
/// as their `jti` so they share a single session row.
pub fn generate_token_pair(
    user_id: Uuid,
    username: &str,
    secret: &str,
    access_ttl: u64,
    refresh_ttl: u64,
    two_fa_verified: bool,
    email_verified: bool,
) -> Result<TokenPair, jsonwebtoken::errors::Error> {
    let session_id = Uuid::new_v4();
    Ok(TokenPair {
        access_token: generate_access_token(
            user_id, username, secret, access_ttl, session_id, two_fa_verified, email_verified,
        )?,
        refresh_token: generate_refresh_token(user_id, username, secret, refresh_ttl, session_id)?,
        expires_in: access_ttl,
        token_type: "Bearer".to_string(),
        session_id,
    })
}

/// Generate a short-lived (5-minute) MFA challenge token.
///
/// Issued after password verification when the account has `totp_enabled = true`.
/// The client exchanges it for a full token pair by calling `POST /auth/2fa/verify`.
pub fn generate_mfa_challenge_token(
    user_id: Uuid,
    username: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        iat: now.timestamp(),
        exp: (now + Duration::minutes(5)).timestamp(),
        token_type: "mfa_challenge".to_string(),
        jti: Uuid::new_v4().to_string(),
        two_fa_verified: false,
        email_verified: false,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
}


#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-that-is-long-enough-for-hs256-algorithm";

    // ── hash_password / verify_password ──────────────────────────────────────

    #[test]
    fn hash_produces_phc_string() {
        let hash = hash_password("hunter2").expect("hash should succeed");
        // Argon2id PHC strings start with $argon2id$
        assert!(hash.starts_with("$argon2id$"), "expected PHC format, got: {hash}");
    }

    #[test]
    fn correct_password_verifies() {
        let hash = hash_password("correct horse battery staple").unwrap();
        let ok = verify_password("correct horse battery staple", &hash).unwrap();
        assert!(ok);
    }

    #[test]
    fn wrong_password_does_not_verify() {
        let hash = hash_password("correct horse battery staple").unwrap();
        let ok = verify_password("wrong password", &hash).unwrap();
        assert!(!ok);
    }

    #[test]
    fn same_password_produces_different_hashes() {
        // Salt is random, so hashes must differ
        let h1 = hash_password("password").unwrap();
        let h2 = hash_password("password").unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn empty_password_is_hashable() {
        let hash = hash_password("").unwrap();
        assert!(verify_password("", &hash).unwrap());
        assert!(!verify_password("not empty", &hash).unwrap());
    }

    // ── generate_access_token / validate_token ────────────────────────────────

    #[test]
    fn generated_token_decodes_correctly() {
        let user_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let token = generate_access_token(
            user_id, "alice", TEST_SECRET, 3600,
            session_id, false, true,
        ).unwrap();

        let claims = nexus_common::auth::validate_token(&token, TEST_SECRET).unwrap();
        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.username, "alice");
        assert_eq!(claims.token_type, "access");
        assert_eq!(claims.jti, session_id.to_string());
        assert!(!claims.two_fa_verified);
        assert!(claims.email_verified);
    }

    #[test]
    fn wrong_secret_is_rejected() {
        let token = generate_access_token(
            Uuid::new_v4(), "bob", TEST_SECRET, 3600,
            Uuid::new_v4(), false, false,
        ).unwrap();

        let err = nexus_common::auth::validate_token(&token, "wrong-secret");
        assert!(err.is_err(), "should reject token signed with different secret");
    }

    #[test]
    fn expired_token_is_rejected() {
        // TTL of 0 seconds — already expired by the time we validate
        let token = generate_access_token(
            Uuid::new_v4(), "carol", TEST_SECRET, 0,
            Uuid::new_v4(), false, false,
        ).unwrap();

        // Give it a moment to definitely expire
        std::thread::sleep(std::time::Duration::from_secs(1));
        let err = nexus_common::auth::validate_token(&token, TEST_SECRET);
        assert!(err.is_err(), "should reject expired token");
    }

    #[test]
    fn malformed_token_is_rejected() {
        let err = nexus_common::auth::validate_token("not.a.token", TEST_SECRET);
        assert!(err.is_err());

        let err2 = nexus_common::auth::validate_token("", TEST_SECRET);
        assert!(err2.is_err());
    }

    // ── generate_token_pair ───────────────────────────────────────────────────

    #[test]
    fn token_pair_access_and_refresh_share_session_id() {
        let user_id = Uuid::new_v4();
        let pair = generate_token_pair(
            user_id, "dave", TEST_SECRET, 900, 86400, false, false,
        ).unwrap();

        let access = nexus_common::auth::validate_token(&pair.access_token, TEST_SECRET).unwrap();
        let refresh = nexus_common::auth::validate_token(&pair.refresh_token, TEST_SECRET).unwrap();

        assert_eq!(access.jti, refresh.jti,
            "access and refresh tokens in a pair must share the same jti (session_id)");
        assert_eq!(access.jti, pair.session_id.to_string());
    }

    #[test]
    fn token_pair_has_correct_token_types() {
        let pair = generate_token_pair(
            Uuid::new_v4(), "eve", TEST_SECRET, 900, 86400, false, false,
        ).unwrap();

        let access = nexus_common::auth::validate_token(&pair.access_token, TEST_SECRET).unwrap();
        let refresh = nexus_common::auth::validate_token(&pair.refresh_token, TEST_SECRET).unwrap();

        assert_eq!(access.token_type, "access");
        assert_eq!(refresh.token_type, "refresh");
    }

    #[test]
    fn token_pair_propagates_two_fa_flag_to_access_token_only() {
        let pair = generate_token_pair(
            Uuid::new_v4(), "frank", TEST_SECRET, 900, 86400,
            true,  // two_fa_verified
            false, // email_verified
        ).unwrap();

        let access = nexus_common::auth::validate_token(&pair.access_token, TEST_SECRET).unwrap();
        let refresh = nexus_common::auth::validate_token(&pair.refresh_token, TEST_SECRET).unwrap();

        assert!(access.two_fa_verified,  "access token should carry two_fa_verified=true");
        assert!(!refresh.two_fa_verified, "refresh token never carries two_fa_verified");
    }

    #[test]
    fn token_pair_reports_correct_expires_in() {
        let pair = generate_token_pair(
            Uuid::new_v4(), "grace", TEST_SECRET, 3600, 86400, false, false,
        ).unwrap();
        assert_eq!(pair.expires_in, 3600);
        assert_eq!(pair.token_type, "Bearer");
    }

    #[test]
    fn different_pairs_have_distinct_session_ids() {
        let uid = Uuid::new_v4();
        let pair1 = generate_token_pair(uid, "u", TEST_SECRET, 900, 86400, false, false).unwrap();
        let pair2 = generate_token_pair(uid, "u", TEST_SECRET, 900, 86400, false, false).unwrap();
        assert_ne!(pair1.session_id, pair2.session_id);
    }

    // ── generate_mfa_challenge_token ──────────────────────────────────────────

    #[test]
    fn mfa_challenge_token_has_correct_type() {
        let user_id = Uuid::new_v4();
        let token = generate_mfa_challenge_token(user_id, "helen", TEST_SECRET).unwrap();
        let claims = nexus_common::auth::validate_token(&token, TEST_SECRET).unwrap();
        assert_eq!(claims.token_type, "mfa_challenge");
        assert_eq!(claims.sub, user_id.to_string());
        assert!(!claims.two_fa_verified);
    }
}
