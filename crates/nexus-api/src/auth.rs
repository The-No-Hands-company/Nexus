//! Authentication — JWT-based, privacy-first.
//!
//! No phone numbers. No government ID. No facial recognition. No age estimation.
//! Just a username, optional email, and a strong password.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT claims embedded in access tokens.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// Issued at
    pub iat: i64,
    /// Expiration
    pub exp: i64,
    /// Token type ("access" or "refresh")
    pub token_type: String,
}

/// Token pair returned on login/register.
#[derive(Debug, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

/// Hash a password using Argon2id (the gold standard for password hashing).
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Verify a password against an Argon2id hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(Argon2::default()
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

/// Validate and decode a JWT token.
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}
