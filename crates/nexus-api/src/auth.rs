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

