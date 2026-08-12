//! Verification of ecosystem identity tokens minted by Nexus-Auth.
//!
//! The ecosystem proxy authenticates the browser, then mints a short-lived
//! RS256 token describing the signed-in user and forwards it to the app as
//! `X-Nexus-Identity`. This module is the app side of that contract: it turns
//! that header into a verified [`IdentityClaims`], or refuses.
//!
//! Everything here is about refusing correctly. The header arrives on an
//! ordinary HTTP request, so the *only* thing separating a real user from an
//! attacker typing a header by hand is the signature check below — which means
//! each rejection path is load-bearing and each one has a test.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// The issuer Nexus-Auth stamps on every token it mints.
pub const IDENTITY_ISSUER: &str = "nexus-auth";

/// The `typ` that marks a token as describing a *user*.
///
/// Auth signs its service-to-service tokens with the same key, so the
/// signature alone does not tell the two apart — this claim does. Without
/// checking it, any service token would authenticate as whatever user its
/// `sub` happened to name.
pub const IDENTITY_TYP: &str = "identity";

/// How long keys are trusted before a background-refresh is due.
const DEFAULT_TTL: Duration = Duration::from_secs(3600);

/// Floor between two on-demand refreshes triggered by an unknown `kid`.
///
/// Unknown kids are attacker-controllable: the header is unauthenticated at
/// the point we read it, so anyone can send a random `kid` and, without this,
/// make us hit Auth's JWKS endpoint once per request. Rotation still lands
/// promptly; a flood costs one fetch per interval.
const MIN_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

/// The claim set Nexus-Auth puts in an identity token.
///
/// Mirrors `buildIdentityClaims` in `apps/Nexus-Auth/src/identity.ts`. `iat`
/// and `jti` are minted but not needed here, and unknown claims are ignored,
/// so Auth can add fields without breaking this.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub email: String,
    pub username: String,
    pub role: String,
    pub typ: String,
    pub exp: usize,
}

/// Why a token was refused.
///
/// Deliberately granular for logging and tests. Callers should *not* pass
/// these back to the client verbatim: the distinction between "unknown key"
/// and "bad signature" is useful to an operator and useful to an attacker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityError {
    /// Not a JWT, or not RS256, or the claim set does not deserialise.
    Malformed,
    /// Header carried no `kid`, or a `kid` Auth's JWKS does not publish.
    UnknownKey,
    /// The signature does not match the key it names.
    BadSignature,
    /// Minted for a different host — a replay from another app.
    WrongAudience,
    /// Issued by something other than Nexus-Auth.
    WrongIssuer,
    /// `exp` has passed.
    Expired,
    /// A validly signed token that is not a user identity (e.g. a service token).
    WrongType,
    /// Auth's JWKS could not be fetched, so nothing can be verified.
    JwksUnavailable,
}

impl std::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Malformed => "malformed identity token",
            Self::UnknownKey => "identity token names an unknown key",
            Self::BadSignature => "identity token signature does not verify",
            Self::WrongAudience => "identity token was minted for another audience",
            Self::WrongIssuer => "identity token was not issued by nexus-auth",
            Self::Expired => "identity token has expired",
            Self::WrongType => "token is not a user identity token",
            Self::JwksUnavailable => "auth jwks is unavailable",
        };
        f.write_str(s)
    }
}

impl std::error::Error for IdentityError {}

/// One RSA key as published by Auth's JWKS endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct Jwk {
    pub kid: String,
    pub kty: String,
    /// Modulus, base64url, no padding.
    pub n: String,
    /// Exponent, base64url, no padding.
    pub e: String,
    #[serde(default)]
    pub alg: Option<String>,
}

/// The document served at `/api/v1/auth/oauth/jwks`.
#[derive(Debug, Clone, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

#[derive(Default)]
struct CachedKeys {
    keys: HashMap<String, DecodingKey>,
    /// When the cache was last *successfully* filled.
    fetched_at: Option<Instant>,
    /// When a fetch was last *attempted*, successful or not. Drives the floor
    /// on on-demand refreshes; using fetched_at would let a permanently failing
    /// endpoint be retried on every request.
    attempted_at: Option<Instant>,
}

/// Auth's signing keys, cached, with refresh on rotation.
pub struct JwksCache {
    jwks_url: String,
    ttl: Duration,
    http: reqwest::Client,
    inner: RwLock<CachedKeys>,
}

impl JwksCache {
    pub fn new(jwks_url: impl Into<String>) -> Self {
        Self {
            jwks_url: jwks_url.into(),
            ttl: DEFAULT_TTL,
            http: reqwest::Client::new(),
            inner: RwLock::new(CachedKeys::default()),
        }
    }

    /// Build the JWKS URL from Auth's internal base URL.
    pub fn from_auth_base_url(base: &str) -> Self {
        Self::new(format!(
            "{}/api/v1/auth/oauth/jwks",
            base.trim_end_matches('/')
        ))
    }

    /// Fill the cache from a JWKS document without any HTTP.
    ///
    /// Used by the tests, and usable in deployments that would rather preload
    /// keys than have the first request pay for a fetch.
    pub async fn seed(&self, jwks: &Jwks) {
        let parsed = parse_keys(jwks);
        let mut guard = self.inner.write().await;
        guard.keys = parsed;
        guard.fetched_at = Some(Instant::now());
        guard.attempted_at = Some(Instant::now());
    }

    /// Verify `token` and return its claims, or say why not.
    ///
    /// `audience` is the host the app is serving as. It must be checked: a
    /// token minted for one app is otherwise perfectly valid at another, so
    /// any app in the ecosystem could replay its users' tokens sideways.
    pub async fn verify(
        &self,
        token: &str,
        audience: &str,
    ) -> Result<IdentityClaims, IdentityError> {
        let header = decode_header(token).map_err(|_| IdentityError::Malformed)?;
        if header.alg != Algorithm::RS256 {
            // Never take the algorithm from the token itself beyond rejecting
            // it: accepting whatever it names is how "alg: none" and
            // HMAC-with-the-public-key forgeries work.
            return Err(IdentityError::Malformed);
        }
        let kid = header.kid.ok_or(IdentityError::UnknownKey)?;

        let key = match self.key_for(&kid).await {
            Some(k) => k,
            None => {
                // An unknown kid is the ordinary signature of a key rotation,
                // so refresh once and look again before refusing.
                self.refresh_if_allowed().await?;
                self.key_for(&kid).await.ok_or(IdentityError::UnknownKey)?
            }
        };

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        // No leeway. These tokens live 120s and travel one hop inside this
        // host; there are no clocks here to disagree with.
        validation.leeway = 0;
        validation.set_audience(&[audience]);
        validation.set_issuer(&[IDENTITY_ISSUER]);

        let data = decode::<IdentityClaims>(token, &key, &validation).map_err(map_jwt_error)?;

        if data.claims.typ != IDENTITY_TYP {
            return Err(IdentityError::WrongType);
        }
        Ok(data.claims)
    }

    async fn key_for(&self, kid: &str) -> Option<DecodingKey> {
        let guard = self.inner.read().await;
        let fresh = guard
            .fetched_at
            .map(|t| t.elapsed() < self.ttl)
            .unwrap_or(false);
        if !fresh {
            return None;
        }
        guard.keys.get(kid).cloned()
    }

    /// Fetch the JWKS, unless we tried too recently.
    async fn refresh_if_allowed(&self) -> Result<(), IdentityError> {
        {
            let guard = self.inner.read().await;
            if let Some(t) = guard.attempted_at {
                if t.elapsed() < MIN_REFRESH_INTERVAL {
                    // Recently attempted; treat as "no such key" rather than
                    // hammering Auth once per forged request.
                    return Ok(());
                }
            }
        }
        {
            let mut guard = self.inner.write().await;
            guard.attempted_at = Some(Instant::now());
        }

        let resp = self
            .http
            .get(&self.jwks_url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|_| IdentityError::JwksUnavailable)?;
        if !resp.status().is_success() {
            return Err(IdentityError::JwksUnavailable);
        }
        let jwks: Jwks = resp
            .json()
            .await
            .map_err(|_| IdentityError::JwksUnavailable)?;

        let parsed = parse_keys(&jwks);
        let mut guard = self.inner.write().await;
        guard.keys = parsed;
        guard.fetched_at = Some(Instant::now());
        Ok(())
    }
}

fn parse_keys(jwks: &Jwks) -> HashMap<String, DecodingKey> {
    jwks.keys
        .iter()
        .filter(|k| k.kty == "RSA")
        .filter_map(|k| {
            DecodingKey::from_rsa_components(&k.n, &k.e)
                .ok()
                .map(|d| (k.kid.clone(), d))
        })
        .collect()
}

fn map_jwt_error(err: jsonwebtoken::errors::Error) -> IdentityError {
    use jsonwebtoken::errors::ErrorKind;
    match err.kind() {
        ErrorKind::ExpiredSignature => IdentityError::Expired,
        ErrorKind::InvalidAudience => IdentityError::WrongAudience,
        ErrorKind::InvalidIssuer => IdentityError::WrongIssuer,
        ErrorKind::InvalidSignature => IdentityError::BadSignature,
        _ => IdentityError::Malformed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    /// A throwaway RSA keypair, generated once per test process.
    ///
    /// Generated rather than embedded on purpose: a PEM in the source of a
    /// public repo trips every secret scanner that ever looks at it, and the
    /// habit of waving those warnings through is worse than the half second
    /// this costs. Nothing here signs anything real.
    struct TestKey {
        pem: String,
        /// Modulus, base64url — the `n` an equivalent JWKS entry would carry.
        n: String,
        /// Exponent, base64url.
        e: String,
    }

    fn test_key() -> &'static TestKey {
        use base64::Engine;
        use rsa::pkcs8::{EncodePrivateKey, LineEnding};
        use rsa::traits::PublicKeyParts;

        static KEY: std::sync::OnceLock<TestKey> = std::sync::OnceLock::new();
        KEY.get_or_init(|| {
            let key = rsa::RsaPrivateKey::new(&mut rand_core::OsRng, 2048)
                .expect("test keypair generates");
            let b64 = |b: Vec<u8>| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b);
            TestKey {
                pem: key
                    .to_pkcs8_pem(LineEnding::LF)
                    .expect("test key encodes as pkcs8")
                    .to_string(),
                n: b64(key.n().to_bytes_be()),
                e: b64(key.e().to_bytes_be()),
            }
        })
    }

    const TEST_KID: &str = "test-key-1";
    const AUDIENCE: &str = "chat.tnhc.dev";

    async fn seeded_cache() -> JwksCache {
        // A URL that would fail loudly if anything reached for it. Nothing in
        // these tests should: the cache is filled directly.
        let cache = JwksCache::new("http://127.0.0.1:1/never-fetched");
        cache
            .seed(&Jwks {
                keys: vec![Jwk {
                    kid: TEST_KID.to_string(),
                    kty: "RSA".to_string(),
                    n: test_key().n.clone(),
                    e: test_key().e.clone(),
                    alg: Some("RS256".to_string()),
                }],
            })
            .await;
        cache
    }

    fn now() -> usize {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
    }

    fn claims(aud: &str, typ: &str, exp: usize) -> IdentityClaims {
        IdentityClaims {
            iss: IDENTITY_ISSUER.to_string(),
            sub: "user-1".to_string(),
            aud: aud.to_string(),
            email: "founder@tnhc.dev".to_string(),
            username: "founder".to_string(),
            role: "owner".to_string(),
            typ: typ.to_string(),
            exp,
        }
    }

    fn sign_with_kid(c: &IdentityClaims, kid: &str) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.to_string());
        let key = EncodingKey::from_rsa_pem(test_key().pem.as_bytes()).expect("test key parses");
        encode(&header, c, &key).expect("test token signs")
    }

    fn sign(c: &IdentityClaims) -> String {
        sign_with_kid(c, TEST_KID)
    }

    #[tokio::test]
    async fn accepts_a_valid_token_for_this_audience() {
        let cache = seeded_cache().await;
        let token = sign(&claims(AUDIENCE, IDENTITY_TYP, now() + 120));

        let got = cache.verify(&token, AUDIENCE).await.expect("verifies");

        assert_eq!(got.sub, "user-1");
        assert_eq!(got.username, "founder");
        assert_eq!(got.role, "owner");
        assert_eq!(got.aud, AUDIENCE);
    }

    #[tokio::test]
    async fn rejects_a_token_minted_for_another_audience() {
        // The sideways-replay case. Draw's proxy hop mints a perfectly valid,
        // correctly signed token; it must not open a session in Chat.
        let cache = seeded_cache().await;
        let token = sign(&claims("draw.tnhc.dev", IDENTITY_TYP, now() + 120));

        assert_eq!(
            cache.verify(&token, AUDIENCE).await.unwrap_err(),
            IdentityError::WrongAudience
        );
    }

    #[tokio::test]
    async fn rejects_an_expired_token() {
        let cache = seeded_cache().await;
        let token = sign(&claims(AUDIENCE, IDENTITY_TYP, now() - 1));

        assert_eq!(
            cache.verify(&token, AUDIENCE).await.unwrap_err(),
            IdentityError::Expired
        );
    }

    #[tokio::test]
    async fn rejects_a_token_that_expired_exactly_now() {
        // Guards the leeway setting. jsonwebtoken's default leeway is 60s,
        // which would quietly accept tokens for half again their lifetime.
        let cache = seeded_cache().await;
        let token = sign(&claims(AUDIENCE, IDENTITY_TYP, now() - 30));

        assert_eq!(
            cache.verify(&token, AUDIENCE).await.unwrap_err(),
            IdentityError::Expired
        );
    }

    #[tokio::test]
    async fn rejects_a_tampered_payload() {
        let cache = seeded_cache().await;
        let token = sign(&claims(AUDIENCE, IDENTITY_TYP, now() + 120));

        // Re-encode the payload with an escalated role and keep the original
        // signature — the forgery an attacker would actually attempt.
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        let forged_payload = {
            use base64::Engine;
            let mut c = claims(AUDIENCE, IDENTITY_TYP, now() + 120);
            c.role = "admin".to_string();
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(serde_json::to_vec(&c).unwrap())
        };
        let forged = format!("{}.{}.{}", parts[0], forged_payload, parts[2]);

        assert_eq!(
            cache.verify(&forged, AUDIENCE).await.unwrap_err(),
            IdentityError::BadSignature
        );
    }

    #[tokio::test]
    async fn rejects_an_unknown_kid() {
        let cache = seeded_cache().await;
        let token = sign_with_kid(&claims(AUDIENCE, IDENTITY_TYP, now() + 120), "not-a-key");

        // The refresh attempt hits an unroutable URL and fails; the answer is
        // still a clean refusal rather than an accept or a panic.
        let err = cache.verify(&token, AUDIENCE).await.unwrap_err();
        assert!(
            err == IdentityError::UnknownKey || err == IdentityError::JwksUnavailable,
            "expected a refusal, got {err:?}"
        );
    }

    #[tokio::test]
    async fn rejects_a_non_identity_typ() {
        // Auth signs service tokens with this same key, so the signature check
        // passes. Only typ stops a service token becoming a user session.
        let cache = seeded_cache().await;
        let token = sign(&claims(AUDIENCE, "service", now() + 120));

        assert_eq!(
            cache.verify(&token, AUDIENCE).await.unwrap_err(),
            IdentityError::WrongType
        );
    }

    #[tokio::test]
    async fn rejects_a_foreign_issuer() {
        let cache = seeded_cache().await;
        let mut c = claims(AUDIENCE, IDENTITY_TYP, now() + 120);
        c.iss = "somebody-else".to_string();
        let token = sign(&c);

        assert_eq!(
            cache.verify(&token, AUDIENCE).await.unwrap_err(),
            IdentityError::WrongIssuer
        );
    }

    #[tokio::test]
    async fn rejects_a_header_with_no_kid() {
        let cache = seeded_cache().await;
        let header = Header::new(Algorithm::RS256); // no kid
        let key = EncodingKey::from_rsa_pem(test_key().pem.as_bytes()).unwrap();
        let token = encode(&header, &claims(AUDIENCE, IDENTITY_TYP, now() + 120), &key).unwrap();

        assert_eq!(
            cache.verify(&token, AUDIENCE).await.unwrap_err(),
            IdentityError::UnknownKey
        );
    }

    #[tokio::test]
    async fn rejects_a_non_rs256_algorithm() {
        // The classic downgrade: sign with HMAC using the public modulus as
        // the shared secret. If the algorithm came from the token, this would
        // verify.
        let cache = seeded_cache().await;
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(TEST_KID.to_string());
        let token = encode(
            &header,
            &claims(AUDIENCE, IDENTITY_TYP, now() + 120),
            &EncodingKey::from_secret(test_key().n.as_bytes()),
        )
        .unwrap();

        assert_eq!(
            cache.verify(&token, AUDIENCE).await.unwrap_err(),
            IdentityError::Malformed
        );
    }

    #[tokio::test]
    async fn rejects_garbage() {
        let cache = seeded_cache().await;
        assert_eq!(
            cache.verify("not-a-jwt", AUDIENCE).await.unwrap_err(),
            IdentityError::Malformed
        );
    }

    #[tokio::test]
    async fn an_empty_cache_does_not_accept_anything() {
        // Never-seeded, unreachable JWKS: the fail-closed case.
        let cache = JwksCache::new("http://127.0.0.1:1/never-fetched");
        let token = sign(&claims(AUDIENCE, IDENTITY_TYP, now() + 120));

        assert!(cache.verify(&token, AUDIENCE).await.is_err());
    }

    #[test]
    fn jwks_parsing_skips_keys_it_cannot_use() {
        let jwks = Jwks {
            keys: vec![
                Jwk {
                    kid: "ec-key".into(),
                    kty: "EC".into(),
                    n: "irrelevant".into(),
                    e: "AQAB".into(),
                    alg: None,
                },
                Jwk {
                    kid: TEST_KID.into(),
                    kty: "RSA".into(),
                    n: test_key().n.clone(),
                    e: test_key().e.clone(),
                    alg: Some("RS256".into()),
                },
            ],
        };

        let keys = parse_keys(&jwks);

        assert_eq!(keys.len(), 1);
        assert!(keys.contains_key(TEST_KID));
    }

    /// End-to-end against a running Nexus-Auth, with a token Auth really minted.
    ///
    /// Ignored by default: it needs the live service, and identity tokens last
    /// 120 seconds, so it cannot be part of an ordinary run. It exists because
    /// every other test here signs its own tokens with a key of its own
    /// choosing — which proves this module is self-consistent, not that it
    /// agrees with Auth. Contract drift (an `aud` array, a renamed claim, a
    /// different `typ`) would sail past the whole suite above and fail here.
    ///
    /// ```text
    /// S=$(curl -s -X POST -H 'content-type: application/json' \
    ///       -d '{"username":"founder","password":"…"}' \
    ///       http://127.0.0.1:4310/api/v1/auth/login | jq -r .token)
    /// export NEXUS_TEST_IDENTITY_TOKEN=$(curl -s -X POST \
    ///       -H "Authorization: Bearer $S" -H 'content-type: application/json' \
    ///       -d '{"audience":"chat.tnhc.dev"}' \
    ///       http://127.0.0.1:4310/api/v1/auth/identity-token | jq -r .token)
    /// cargo test -p nexus-api identity::tests::verifies_a_real_token -- --ignored
    /// ```
    #[tokio::test]
    #[ignore = "needs a running Nexus-Auth and a fresh NEXUS_TEST_IDENTITY_TOKEN"]
    async fn verifies_a_real_token_from_a_running_auth() {
        let token = std::env::var("NEXUS_TEST_IDENTITY_TOKEN")
            .expect("set NEXUS_TEST_IDENTITY_TOKEN (see the doc comment)");
        let auth_url = std::env::var("NEXUS_AUTH_INTERNAL_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:4310".to_string());
        let audience =
            std::env::var("NEXUS_TEST_IDENTITY_AUDIENCE").unwrap_or_else(|_| AUDIENCE.to_string());

        // No seeding: this fetches Auth's real JWKS over HTTP, which also
        // exercises the fetch-and-parse path the unit tests deliberately skip.
        let cache = JwksCache::from_auth_base_url(&auth_url);
        let claims = cache
            .verify(&token, &audience)
            .await
            .expect("a freshly minted token verifies against the live jwks");

        assert_eq!(claims.iss, IDENTITY_ISSUER);
        assert_eq!(claims.typ, IDENTITY_TYP);
        assert_eq!(claims.aud, audience);
        assert!(!claims.sub.is_empty(), "sub must identify the user");
        assert!(!claims.username.is_empty(), "username must be present");
        assert!(!claims.role.is_empty(), "role must be present");

        // And the same token must not satisfy a different app.
        assert_eq!(
            cache.verify(&token, "somewhere-else.tnhc.dev").await,
            Err(IdentityError::WrongAudience)
        );
    }

    #[test]
    fn from_auth_base_url_builds_the_jwks_path() {
        let c = JwksCache::from_auth_base_url("http://127.0.0.1:4310/");
        assert_eq!(c.jwks_url, "http://127.0.0.1:4310/api/v1/auth/oauth/jwks");
    }
}
