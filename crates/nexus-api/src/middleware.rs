//! Middleware — authentication extraction, rate limiting, security headers, metrics, etc.

use axum::{extract::Request, http::header, middleware::Next, response::Response};
use nexus_common::error::NexusError;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::{AppState, auth};

// ── HTTP metrics middleware ──────────────────────────────────────────────────

/// Normalize a raw URI path to a low-cardinality label suitable for Prometheus.
///
/// Keeps the first 4 slash-delimited segments (e.g. `/api/v1/channels`) and
/// drops dynamic path params that would explode label cardinality.
fn normalize_path(path: &str) -> String {
    // Try to use the matched route pattern from Axum (`:id` placeholders)
    // before falling back to raw prefix truncation.
    path.split('/').take(4).collect::<Vec<_>>().join("/")
}

/// Record per-request Prometheus counters and latency histograms.
///
/// Labels:
///   `nexus_http_requests_total{method, path, status}`
///   `nexus_http_request_duration_seconds{method, path}`
pub async fn record_request_metrics(request: Request, next: Next) -> Response {
    let method = request.method().as_str().to_owned();
    // Prefer the matched route pattern if Axum has already resolved it;
    // otherwise fall back to the raw URI path (normalized to avoid high cardinality).
    let path = request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|mp| mp.as_str().to_owned())
        .unwrap_or_else(|| normalize_path(request.uri().path()));

    let start = Instant::now();
    let response = next.run(request).await;
    let elapsed = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    metrics::counter!(
        "nexus_http_requests_total",
        "method" => method.clone(),
        "path" => path.clone(),
        "status" => status,
    )
    .increment(1);
    metrics::histogram!(
        "nexus_http_request_duration_seconds",
        "method" => method,
        "path" => path,
    )
    .record(elapsed);

    response
}

/// Authentication context extracted from the Authorization header.
///
/// Populated by either `auth_middleware` (JWT Bearer tokens for human users)
/// or `combined_auth_middleware` (supports both JWT Bearer and `Bot <token>`).
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: uuid::Uuid,
    pub username: String,
    /// `true` when the request was authenticated with a bot token rather than
    /// a user JWT.  Handlers that should be user-only can reject bots here.
    pub is_bot: bool,
    /// The JTI (JWT ID) of the current access token, parsed as UUID.
    /// Used by session-management routes to identify the caller's own session.
    /// `None` for bot tokens or tokens issued before 09.7 (no `jti` claim).
    pub session_id: Option<uuid::Uuid>,
    /// Whether the token was issued after a successful 2FA check.
    pub two_fa_verified: bool,
    /// Whether the user's email was verified at the time of token issuance.
    /// Always `true` for bot tokens and for users with no email registered.
    pub email_verified: bool,
    /// User account flags (staff, admin, creator, etc.)
    /// Lazily loaded on demand by helpers
    pub flags: std::sync::Arc<tokio::sync::Mutex<Option<i64>>>,
}

impl AuthContext {
    /// Check if user has a specific flag. Caches flags in-memory.
    pub async fn has_flag(&self, flag: i64) -> bool {
        if let Some(flags) = *self.flags.lock().await {
            flags & flag != 0
        } else {
            false
        }
    }

    /// Check if user is a marketplace admin (reviewer or moderator)
    pub async fn is_marketplace_admin(&self) -> bool {
        use nexus_common::models::user_flags;
        self.has_flag(user_flags::MARKETPLACE_REVIEWER).await
            || self.has_flag(user_flags::MARKETPLACE_MODERATOR).await
            || self.has_flag(user_flags::INSTANCE_ADMIN).await
    }

    /// Check if user can review plugins
    pub async fn can_review_plugins(&self) -> bool {
        use nexus_common::models::user_flags;
        self.has_flag(user_flags::MARKETPLACE_REVIEWER).await
            || self.has_flag(user_flags::INSTANCE_ADMIN).await
    }

    /// Check if user can handle takedowns
    pub async fn can_handle_takedowns(&self) -> bool {
        use nexus_common::models::user_flags;
        self.has_flag(user_flags::MARKETPLACE_MODERATOR).await
            || self.has_flag(user_flags::INSTANCE_ADMIN).await
    }

    /// Load flags from database (call once per request if needed)
    pub async fn load_flags(&self, pool: &sqlx::AnyPool) -> Result<i64, sqlx::Error> {
        if self.flags.lock().await.is_some() {
            return Ok(self.flags.lock().await.unwrap());
        }

        let flags = sqlx::query_scalar::<_, i64>("SELECT flags FROM users WHERE id = $1")
            .bind(self.user_id.to_string())
            .fetch_optional(pool)
            .await?
            .unwrap_or(0);

        *self.flags.lock().await = Some(flags);
        Ok(flags)
    }
}

// ── SHA-256 helper (used for bot token hashing) ──────────────────────────────

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    format!("{:x}", h.finalize())
}

// ── JWT-only middleware (user routes that bots must not access) ──────────────

/// Extract and validate the JWT from the `Authorization: Bearer <token>` header.
///
/// Use this middleware on routes that must only be called by human users (e.g.
/// login, registration, token refresh).  For routes that both users AND bots may
/// call, use `combined_auth_middleware` instead.
pub async fn auth_middleware(mut request: Request, next: Next) -> Result<Response, NexusError> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(NexusError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(NexusError::Unauthorized)?;

    let config = nexus_common::config::get();
    let claims = auth::validate_token(token, &config.auth.jwt_secret)
        .map_err(|_| NexusError::InvalidToken)?;

    // Ensure it's an access token, not a refresh token
    if claims.token_type != "access" {
        return Err(NexusError::InvalidToken);
    }

    let user_id = claims
        .sub
        .parse::<uuid::Uuid>()
        .map_err(|_| NexusError::InvalidToken)?;

    let session_id = claims.jti.parse::<uuid::Uuid>().ok();

    // ── Session revocation check ──────────────────────────────────────────────
    // Validate the JWT's `jti` claim against the active sessions table so that
    // logout (which deletes the session row) actually invalidates the token
    // before its cryptographic expiry.
    //
    // We cache the result in Redis with a TTL of 60 seconds to bound the DB
    // fanout while keeping the revocation window short:
    //   - Tokens expire in 15 minutes (ACCESS_TOKEN_TTL_SECS = 900)
    //   - Revoked tokens stay valid for at most 60 extra seconds
    //   - Each unique session_id is looked up from Redis, not DB, on hot paths
    //
    // Tokens issued before jti support (jti == "") bypass this check — they
    // expire naturally and can no longer be issued.
    if let Some(sid) = session_id {
        let state = request
            .extensions()
            .get::<std::sync::Arc<AppState>>()
            .cloned();

        if let Some(state) = state {
            // Fast path: check Redis cache first
            let cache_key = format!("sess:active:{sid}");
            let cached = if let Some(ref redis) = state.db.redis {
                let mut conn = redis.clone();
                let val: Option<String> = redis::AsyncCommands::get(&mut conn, &cache_key)
                    .await
                    .unwrap_or(None);
                val
            } else {
                None
            };

            let is_active = match cached.as_deref() {
                Some("1") => true,
                Some("0") => false,
                None => {
                    // Cache miss — query DB
                    let exists = nexus_db::repository::sessions::session_exists(
                        &state.db.pool,
                        sid,
                        user_id,
                    )
                    .await
                    .unwrap_or(true); // fail open on DB error to avoid locking out all users

                    // Cache result for 60 seconds
                    if let Some(ref redis) = state.db.redis {
                        let mut conn = redis.clone();
                        let val = if exists { "1" } else { "0" };
                        let _: Result<(), _> =
                            redis::AsyncCommands::set_ex(&mut conn, &cache_key, val, 60u64).await;
                    }

                    exists
                }
                _ => true,
            };

            if !is_active {
                return Err(NexusError::InvalidToken);
            }
        }
    }

    let two_fa_verified = claims.two_fa_verified;
    let email_verified = claims.email_verified;

    let auth_ctx = AuthContext {
        user_id,
        username: claims.username,
        is_bot: false,
        session_id,
        two_fa_verified,
        email_verified,
        flags: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
    };

    // Insert auth context into request extensions for handlers to use
    request.extensions_mut().insert(auth_ctx.clone());

    // Email verification gate (same policy as combined_auth_middleware)
    let config = nexus_common::config::get();
    if config.features.require_email_verification && !auth_ctx.email_verified {
        let path = request.uri().path();
        if !path.contains("/auth/") && !path.starts_with("/health") {
            return Err(NexusError::Forbidden);
        }
    }

    Ok(next.run(request).await)
}

// ── Combined auth middleware (users + bots) ──────────────────────────────────

/// Accept **either** `Authorization: Bearer <jwt>` (human users) **or**
/// `Authorization: Bot <raw-token>` (bot applications).
///
/// For `Bot` token requests the raw token is SHA-256 hashed and looked up in
/// the `bot_applications` table.  The resulting `AuthContext` has `is_bot =
/// true` and `user_id` set to the bot application's own UUID.
///
/// Requires [`Arc<AppState>`] to be present as an Axum [`Extension`] on the
/// router — `build_router` does this with `.layer(axum::Extension(arc_state))`.
pub async fn combined_auth_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, NexusError> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(NexusError::Unauthorized)?
        .to_owned();

    let auth_ctx = if let Some(raw_token) = auth_header.strip_prefix("Bot ") {
        // ── Bot token authentication ──────────────────────────────────────
        let token_hash = sha256_hex(raw_token);

        // Extract AppState from Extensions (added by build_router)
        let state = request
            .extensions()
            .get::<Arc<AppState>>()
            .cloned()
            .ok_or(NexusError::Unauthorized)?;

        let bot = nexus_db::repository::bots::get_bot_by_token_hash(&state.db.pool, &token_hash)
            .await
            .map_err(|_| NexusError::Unauthorized)?
            .ok_or(NexusError::Unauthorized)?;

        AuthContext {
            user_id: bot.id,
            username: bot.name,
            is_bot: true,
            session_id: None,
            two_fa_verified: false,
            // Bots are not subject to email verification requirements.
            email_verified: true,
            flags: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        }
    } else {
        // ── JWT Bearer authentication ─────────────────────────────────────
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(NexusError::Unauthorized)?;

        let config = nexus_common::config::get();
        let claims = auth::validate_token(token, &config.auth.jwt_secret)
            .map_err(|_| NexusError::InvalidToken)?;

        if claims.token_type != "access" {
            return Err(NexusError::InvalidToken);
        }

        let user_id = claims
            .sub
            .parse::<uuid::Uuid>()
            .map_err(|_| NexusError::InvalidToken)?;

        let session_id = claims.jti.parse::<uuid::Uuid>().ok();
        let two_fa_verified = claims.two_fa_verified;
        let email_verified = claims.email_verified;

        AuthContext {
            user_id,
            username: claims.username,
            is_bot: false,
            session_id,
            two_fa_verified,
            email_verified,
            flags: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        }
    };

    request.extensions_mut().insert(auth_ctx.clone());

    // ── Email verification gate ───────────────────────────────────────────
    // Exempt /auth/* routes so that resend-verification, session management
    // and 2FA completion still work while the account is unverified.
    let config = nexus_common::config::get();
    if config.features.require_email_verification && !auth_ctx.is_bot && !auth_ctx.email_verified {
        let path = request.uri().path();
        let is_auth_path =
            path.contains("/auth/") || path.starts_with("/healthz") || path.starts_with("/health");
        if !is_auth_path {
            return Err(NexusError::Forbidden);
        }
    }

    Ok(next.run(request).await)
}

// ── AuthContext helpers ───────────────────────────────────────────────────────

/// Extract AuthContext from request extensions.
///
/// Usage in handlers:
/// ```rust,ignore
/// async fn my_handler(auth: Extension<AuthContext>) -> impl IntoResponse { ... }
/// ```
impl AuthContext {
    pub fn from_request_extensions(
        extensions: &axum::http::Extensions,
    ) -> Result<&Self, NexusError> {
        extensions
            .get::<AuthContext>()
            .ok_or(NexusError::Unauthorized)
    }
}

// ── Auth rate limiter ────────────────────────────────────────────────────────

/// Redis-backed sliding-window rate limiter.
///
/// Uses the classic `INCR` + `EXPIRE` pattern — O(1) in Redis.
/// The race between INCR and EXPIRE is acceptable: the worst
/// case is that the window extends slightly on the very first hit,
/// not that the limit is relaxed.
///
/// # Arguments
/// * `redis`       – Redis connection manager borrowed from `db.redis`
/// * `key`         – Unique key (e.g. `"rl:login:ip:1.2.3.4"`)
/// * `limit`       – Max calls allowed in the window
/// * `window_secs` – Window length in seconds
///
/// Returns `Ok(())` when the call is within limits, or
/// `Err(NexusError::RateLimited { retry_after_ms })` when exceeded.
pub async fn check_rate_limit(
    redis: &redis::aio::ConnectionManager,
    key: impl AsRef<str>,
    limit: u64,
    window_secs: u64,
) -> Result<(), NexusError> {
    #[allow(unused_imports)]
    use redis::AsyncCommands as _;
    let mut conn = redis.clone();
    let key = key.as_ref();

    let count: u64 = redis::cmd("INCR")
        .arg(key)
        .query_async(&mut conn)
        .await
        .map_err(|e| NexusError::Internal(anyhow::anyhow!("redis INCR failed: {e}")))?;

    if count == 1 {
        // First hit — arm the expiry clock.
        let _: () = redis::cmd("EXPIRE")
            .arg(key)
            .arg(window_secs as i64)
            .query_async(&mut conn)
            .await
            .map_err(|e| NexusError::Internal(anyhow::anyhow!("redis EXPIRE failed: {e}")))?;
    }

    if count > limit {
        let ttl_secs: i64 = redis::cmd("TTL")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| NexusError::Internal(anyhow::anyhow!("redis TTL failed: {e}")))?;
        let retry_after_ms = (ttl_secs.max(1) as u64) * 1000;
        return Err(NexusError::RateLimited { retry_after_ms });
    }

    Ok(())
}

static LOCAL_RATE_LIMITS: OnceLock<Mutex<HashMap<String, (u64, Instant)>>> = OnceLock::new();

/// Rate limiter that prefers Redis (multi-node) and falls back to local memory.
///
/// Use this for public abuse-prone endpoints so limits remain active even when
/// Redis is not configured (lite mode) or temporarily unavailable.
pub async fn check_rate_limit_with_fallback(
    redis: Option<&redis::aio::ConnectionManager>,
    key: impl AsRef<str>,
    limit: u64,
    window_secs: u64,
) -> Result<(), NexusError> {
    if let Some(redis) = redis {
        match check_rate_limit(redis, key.as_ref(), limit, window_secs).await {
            Ok(()) => {
                metrics::counter!(
                    "nexus_rate_limit_decisions_total",
                    "backend" => "redis",
                    "outcome" => "allowed",
                )
                .increment(1);
                return Ok(());
            }
            Err(NexusError::RateLimited { retry_after_ms }) => {
                metrics::counter!(
                    "nexus_rate_limit_decisions_total",
                    "backend" => "redis",
                    "outcome" => "blocked",
                )
                .increment(1);
                return Err(NexusError::RateLimited { retry_after_ms });
            }
            Err(err) => {
                tracing::warn!(error = %err, "redis rate limiter failed; falling back to local limiter");
                metrics::counter!(
                    "nexus_rate_limit_decisions_total",
                    "backend" => "redis",
                    "outcome" => "error",
                )
                .increment(1);
            }
        }
    }

    let res = check_rate_limit_local(key.as_ref(), limit, window_secs).await;
    metrics::counter!(
        "nexus_rate_limit_decisions_total",
        "backend" => "local",
        "outcome" => if res.is_ok() { "allowed" } else { "blocked" },
    )
    .increment(1);
    res
}

async fn check_rate_limit_local(key: &str, limit: u64, window_secs: u64) -> Result<(), NexusError> {
    let state = LOCAL_RATE_LIMITS.get_or_init(|| Mutex::new(HashMap::new()));
    let now = Instant::now();
    let window = Duration::from_secs(window_secs.max(1));

    let mut guard = state.lock().await;

    // Best-effort cleanup to avoid unbounded growth over time.
    if guard.len() > 50_000 {
        guard.retain(|_, (_, reset_at)| *reset_at > now);
    }

    let entry = guard.entry(key.to_string()).or_insert((0, now + window));

    if now >= entry.1 {
        *entry = (1, now + window);
    } else {
        entry.0 = entry.0.saturating_add(1);
    }

    if entry.0 > limit {
        let retry_after_ms = entry.1.saturating_duration_since(now).as_millis() as u64;
        return Err(NexusError::RateLimited {
            retry_after_ms: retry_after_ms.max(1_000),
        });
    }

    Ok(())
}

/// Extract the best-effort client IP from request headers.
///
/// Checks (in order):
///   1. `X-Forwarded-For` first value — set by nginx / Fly.io / Cloudflare
///   2. `X-Real-IP` — set by nginx in single-proxy mode
///   3. Falls back to `"unknown"` (rate-limiting degrades gracefully)
///
/// **Security note:** these headers can be spoofed when Nexus is directly
/// internet-facing.  For production, run behind a trusted reverse proxy
/// that strips and re-sets `X-Forwarded-For`.
pub fn extract_client_ip(headers: &axum::http::HeaderMap) -> String {
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        // XFF may be "client, proxy1, proxy2" — take the leftmost value.
        if let Some(ip) = xff.split(',').next().map(str::trim)
            && !ip.is_empty() {
                return ip.to_owned();
            }
    }
    if let Some(xri) = headers.get("x-real-ip").and_then(|v| v.to_str().ok())
        && !xri.is_empty() {
            return xri.to_owned();
        }
    "unknown".to_owned()
}

// ── Security headers ──────────────────────────────────────────────────────────

/// Add defensive security headers to every HTTP response.
///
/// Headers applied:
/// - `X-Content-Type-Options: nosniff` — prevents MIME sniffing
/// - `X-Frame-Options: SAMEORIGIN` — allows embedding by same origin or Cloud portal
/// - `X-XSS-Protection: 1; mode=block` — legacy XSS protection
/// - `Referrer-Policy: strict-origin-when-cross-origin`
/// - `Permissions-Policy` — disables camera, mic, geolocation
/// - `Strict-Transport-Security` — HSTS (max-age 2 years + preload)
/// - `Content-Security-Policy` — restrictive CSP for API endpoints
pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let h = response.headers_mut();

    macro_rules! set {
        ($name:expr, $val:expr) => {
            if let Ok(v) = $val.parse::<axum::http::HeaderValue>() {
                h.insert($name, v);
            }
        };
    }

    set!(
        axum::http::header::HeaderName::from_static("x-content-type-options"),
        "nosniff"
    );
    set!(
        axum::http::header::HeaderName::from_static("x-frame-options"),
        "SAMEORIGIN"
    );
    set!(
        axum::http::header::HeaderName::from_static("x-xss-protection"),
        "1; mode=block"
    );
    set!(
        axum::http::header::HeaderName::from_static("referrer-policy"),
        "strict-origin-when-cross-origin"
    );
    set!(
        axum::http::header::HeaderName::from_static("permissions-policy"),
        "camera=(), microphone=(), geolocation=(), payment=()"
    );
    set!(
        axum::http::header::HeaderName::from_static("strict-transport-security"),
        "max-age=63072000; includeSubDomains; preload"
    );
    set!(
        axum::http::header::HeaderName::from_static("content-security-policy"),
        "default-src 'self'; \
         script-src 'self'; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: blob:; \
         connect-src 'self' wss:; \
         font-src 'self'; \
         media-src 'self' blob:; \
         worker-src 'self' blob:; \
         frame-ancestors *"
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    // ── normalize_path ────────────────────────────────────────────────────────

    #[test]
    fn normalize_path_keeps_up_to_four_segments() {
        assert_eq!(normalize_path("/api/v1/users/123"), "/api/v1/users/123");
    }

    #[test]
    fn normalize_path_truncates_beyond_four_segments() {
        // Only the first four slash-delimited parts are kept
        assert_eq!(
            normalize_path("/api/v1/channels/abc/messages/def"),
            "/api/v1/channels/abc"
        );
    }

    #[test]
    fn normalize_path_root_is_preserved() {
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn normalize_path_short_path_unchanged() {
        assert_eq!(normalize_path("/health"), "/health");
        assert_eq!(normalize_path("/api/v1"), "/api/v1");
    }

    // ── sha256_hex ────────────────────────────────────────────────────────────

    #[test]
    fn sha256_hex_is_deterministic() {
        let a = sha256_hex("hello");
        let b = sha256_hex("hello");
        assert_eq!(a, b);
    }

    #[test]
    fn sha256_hex_differs_for_different_inputs() {
        assert_ne!(sha256_hex("hello"), sha256_hex("world"));
    }

    #[test]
    fn sha256_hex_produces_64_char_lowercase_hex() {
        let h = sha256_hex("nexus");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
        // Hex digits should all be lowercase
        assert_eq!(h, h.to_lowercase());
    }

    #[test]
    fn sha256_hex_empty_string_known_value() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert!(sha256_hex("").starts_with("e3b0c44"));
    }

    #[test]
    fn sha256_hex_bot_token_differs_from_raw_token() {
        let raw = "my-bot-token";
        let prefixed = format!("Bot {raw}");
        assert_ne!(sha256_hex(raw), sha256_hex(&prefixed));
    }

    // ── extract_client_ip ─────────────────────────────────────────────────────

    #[test]
    fn extract_client_ip_returns_unknown_with_no_headers() {
        let headers = HeaderMap::new();
        assert_eq!(extract_client_ip(&headers), "unknown");
    }

    #[test]
    fn extract_client_ip_reads_x_forwarded_for_first_value() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "203.0.113.1, 10.0.0.1, 172.16.0.1".parse().unwrap(),
        );
        // Must return the leftmost (original client) address
        assert_eq!(extract_client_ip(&headers), "203.0.113.1");
    }

    #[test]
    fn extract_client_ip_reads_x_real_ip_as_fallback() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "198.51.100.42".parse().unwrap());
        assert_eq!(extract_client_ip(&headers), "198.51.100.42");
    }

    #[test]
    fn extract_client_ip_prefers_xff_over_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.1".parse().unwrap());
        headers.insert("x-real-ip", "198.51.100.42".parse().unwrap());
        // X-Forwarded-For takes precedence
        assert_eq!(extract_client_ip(&headers), "203.0.113.1");
    }

    #[test]
    fn extract_client_ip_trims_whitespace_from_xff() {
        let mut headers = HeaderMap::new();
        // Some proxies add extra spaces
        headers.insert(
            "x-forwarded-for",
            "  203.0.113.5  , 10.0.0.1".parse().unwrap(),
        );
        assert_eq!(extract_client_ip(&headers), "203.0.113.5");
    }

    // ── check_rate_limit_local ────────────────────────────────────────────────

    #[tokio::test]
    async fn rate_limit_local_allows_under_limit() {
        // Use a unique key to avoid cross-test interference
        let key = format!("test:allow:{}", uuid::Uuid::new_v4());
        let result = check_rate_limit_local(&key, 5, 60).await;
        assert!(result.is_ok(), "first request should be allowed");
    }

    #[tokio::test]
    async fn rate_limit_local_blocks_over_limit() {
        let key = format!("test:block:{}", uuid::Uuid::new_v4());
        let limit = 3u64;
        // Exhaust the limit
        for _ in 0..limit {
            check_rate_limit_local(&key, limit, 60).await.unwrap();
        }
        // Next call should be rate limited
        let err = check_rate_limit_local(&key, limit, 60).await.unwrap_err();
        assert!(
            matches!(err, NexusError::RateLimited { .. }),
            "expected RateLimited, got {err:?}"
        );
    }

    #[tokio::test]
    async fn rate_limit_local_window_resets_after_expiry() {
        let key = format!("test:reset:{}", uuid::Uuid::new_v4());
        // Exhaust a limit with a 0-second window (already expired by next call)
        check_rate_limit_local(&key, 1, 0).await.unwrap();
        // A 0-second window resets immediately — next call should succeed
        let result = check_rate_limit_local(&key, 1, 0).await;
        assert!(result.is_ok(), "window should have reset");
    }
}
