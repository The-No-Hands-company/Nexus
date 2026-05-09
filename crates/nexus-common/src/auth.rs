//! Shared JWT authentication utilities.
//!
//! Claims and token validation live here so both nexus-api and nexus-gateway
//! can use them without circular dependencies. Password hashing and token
//! generation stay in nexus-api since they're API-specific.

use jsonwebtoken::{DecodingKey, Validation, decode};
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
    /// Token type: "access", "refresh", or `mfa_challenge`
    pub token_type: String,
    /// JWT ID — used as session identifier for revocation lookups.
    /// `#[serde(default)]` ensures tokens issued before 09.7 still decode.
    #[serde(default)]
    pub jti: String,
    /// Whether this token was issued after successful 2FA verification.
    /// `false` for users without 2FA enabled, `true` after TOTP was checked.
    #[serde(default)]
    pub two_fa_verified: bool,
    /// Whether the user's email address was verified at the time of token issuance.
    /// `#[serde(default)]` ensures tokens issued before 09.9 still decode as `false`.
    #[serde(default)]
    pub email_verified: bool,
}

/// Validate and decode a JWT token.
///
/// # Errors
/// Returns an error when token decoding or signature validation fails.
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "a-sufficiently-long-hs256-secret-for-testing-nexus";

    fn make_claims(token_type: &str, exp_offset_secs: i64) -> Claims {
        let now = chrono::Utc::now().timestamp();
        Claims {
            sub: uuid::Uuid::new_v4().to_string(),
            username: "testuser".to_string(),
            iat: now,
            exp: now + exp_offset_secs,
            token_type: token_type.to_string(),
            jti: uuid::Uuid::new_v4().to_string(),
            two_fa_verified: false,
            email_verified: false,
        }
    }

    fn encode_claims(claims: &Claims, secret: &str) -> String {
        use jsonwebtoken::{EncodingKey, Header, encode};
        encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("encoding should succeed in tests")
    }

    // ── validate_token ────────────────────────────────────────────────────────

    #[test]
    fn valid_token_decodes_with_correct_claims() {
        let claims = make_claims("access", 3600);
        let token = encode_claims(&claims, SECRET);

        let decoded = validate_token(&token, SECRET).unwrap();
        assert_eq!(decoded.sub, claims.sub);
        assert_eq!(decoded.username, claims.username);
        assert_eq!(decoded.token_type, "access");
        assert_eq!(decoded.jti, claims.jti);
    }

    #[test]
    fn wrong_secret_rejected() {
        let token = encode_claims(&make_claims("access", 3600), SECRET);
        assert!(validate_token(&token, "wrong-secret").is_err());
    }

    #[test]
    fn empty_secret_rejected_for_token_signed_with_real_secret() {
        let token = encode_claims(&make_claims("access", 3600), SECRET);
        assert!(validate_token(&token, "").is_err());
    }

    #[test]
    fn expired_token_rejected() {
        // exp in the past
        let token = encode_claims(&make_claims("access", -10), SECRET);
        assert!(validate_token(&token, SECRET).is_err());
    }

    #[test]
    fn malformed_jwt_strings_rejected() {
        assert!(validate_token("", SECRET).is_err());
        assert!(validate_token("not.a.token", SECRET).is_err());
        assert!(validate_token("only_one_part", SECRET).is_err());
        // Three parts but garbled
        assert!(validate_token("eyBad.eyBad.eyBad", SECRET).is_err());
    }

    #[test]
    fn refresh_token_type_preserved() {
        let claims = make_claims("refresh", 86400);
        let token = encode_claims(&claims, SECRET);
        let decoded = validate_token(&token, SECRET).unwrap();
        assert_eq!(decoded.token_type, "refresh");
    }

    #[test]
    fn mfa_challenge_token_type_preserved() {
        let claims = make_claims("mfa_challenge", 300);
        let token = encode_claims(&claims, SECRET);
        let decoded = validate_token(&token, SECRET).unwrap();
        assert_eq!(decoded.token_type, "mfa_challenge");
    }

    #[test]
    fn two_fa_verified_flag_round_trips() {
        let mut claims = make_claims("access", 3600);
        claims.two_fa_verified = true;
        let token = encode_claims(&claims, SECRET);
        let decoded = validate_token(&token, SECRET).unwrap();
        assert!(decoded.two_fa_verified);
    }

    #[test]
    fn email_verified_flag_round_trips() {
        let mut claims = make_claims("access", 3600);
        claims.email_verified = true;
        let token = encode_claims(&claims, SECRET);
        let decoded = validate_token(&token, SECRET).unwrap();
        assert!(decoded.email_verified);
    }

    #[test]
    fn serde_default_fields_decode_from_legacy_token_without_them() {
        // Tokens minted before jti/two_fa_verified/email_verified were added
        // must still decode with default values (empty string / false).
        use jsonwebtoken::{EncodingKey, Header, encode};

        #[derive(serde::Serialize)]
        struct LegacyClaims {
            sub: String,
            username: String,
            iat: i64,
            exp: i64,
            token_type: String,
        }

        let now = chrono::Utc::now().timestamp();
        let legacy = LegacyClaims {
            sub: uuid::Uuid::new_v4().to_string(),
            username: "legacy".to_string(),
            iat: now,
            exp: now + 3600,
            token_type: "access".to_string(),
        };

        let token = encode(
            &Header::default(),
            &legacy,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap();

        let decoded = validate_token(&token, SECRET).unwrap();
        assert_eq!(decoded.jti, ""); // serde default
        assert!(!decoded.two_fa_verified); // serde default
        assert!(!decoded.email_verified); // serde default
    }
}
