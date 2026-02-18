//! Shared JWT authentication utilities.
//!
//! Claims and token validation live here so both nexus-api and nexus-gateway
//! can use them without circular dependencies. Password hashing and token
//! generation stay in nexus-api since they're API-specific.

use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// JWT claims embedded in access and refresh tokens.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID as string)
    pub sub: String,
    /// Username
    pub username: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
    /// Token type ("access" or "refresh")
    pub token_type: String,
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
