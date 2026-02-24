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
}

/// Returns the Argon2 instance appropriate for the current build profile.
///
/// Debug builds use minimal parameters (8 MB, 1 iteration) so that
/// login/register complete in ~50 ms instead of 15+ seconds.
/// Release builds use the production-strength defaults.
fn argon2_instance() -> Argon2<'static> {
    #[cfg(debug_assertions)]
    {
        // m_cost = 8 * 1024 KiB, t_cost = 1 iteration, p_cost = 1 lane
        let params = Params::new(8 * 1024, 1, 1, None)
            .expect("valid argon2 debug params");
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
    }
    #[cfg(not(debug_assertions))]
    {
        Argon2::default()
    }
}

/// Hash a password using Argon2id (the gold standard for password hashing).
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
pub fn generate_access_token(
    user_id: Uuid,
    username: &str,
    secret: &str,
    ttl_secs: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        iat: now.timestamp(),
        exp: (now + Duration::seconds(ttl_secs as i64)).timestamp(),
        token_type: "access".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Generate a JWT refresh token (longer-lived).
pub fn generate_refresh_token(
    user_id: Uuid,
    username: &str,
    secret: &str,
    ttl_secs: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        iat: now.timestamp(),
        exp: (now + Duration::seconds(ttl_secs as i64)).timestamp(),
        token_type: "refresh".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Generate both access and refresh tokens.
pub fn generate_token_pair(
    user_id: Uuid,
    username: &str,
    secret: &str,
    access_ttl: u64,
    refresh_ttl: u64,
) -> Result<TokenPair, jsonwebtoken::errors::Error> {
    Ok(TokenPair {
        access_token: generate_access_token(user_id, username, secret, access_ttl)?,
        refresh_token: generate_refresh_token(user_id, username, secret, refresh_ttl)?,
        expires_in: access_ttl,
        token_type: "Bearer".to_string(),
    })
}
