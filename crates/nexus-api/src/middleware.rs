//! Middleware — authentication extraction, rate limiting, security headers, etc.

use axum::{
    extract::Request,
    http::header,
    middleware::Next,
    response::Response,
};
use nexus_common::error::NexusError;
use std::sync::Arc;

use crate::{auth, AppState};

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
pub async fn auth_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, NexusError> {
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

    let auth_ctx = AuthContext {
        user_id,
        username: claims.username,
        is_bot: false,
    };

    // Insert auth context into request extensions for handlers to use
    request.extensions_mut().insert(auth_ctx);

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

        AuthContext {
            user_id,
            username: claims.username,
            is_bot: false,
        }
    };

    request.extensions_mut().insert(auth_ctx);
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
    pub fn from_request_extensions(extensions: &axum::http::Extensions) -> Result<&Self, NexusError> {
        extensions
            .get::<AuthContext>()
            .ok_or(NexusError::Unauthorized)
    }
}

// ── Security headers ──────────────────────────────────────────────────────────

/// Add defensive security headers to every HTTP response.
///
/// Headers applied:
/// - `X-Content-Type-Options: nosniff` — prevents MIME sniffing
/// - `X-Frame-Options: DENY` — prevents clickjacking
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
        "DENY"
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
         frame-ancestors 'none'"
    );

    response
}

