//! Middleware — authentication extraction, rate limiting, security headers, etc.

use axum::{
    extract::Request,
    http::header,
    middleware::Next,
    response::Response,
};
use nexus_common::error::NexusError;

use crate::auth;

/// Authentication context extracted from the Authorization header.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: uuid::Uuid,
    pub username: String,
}

/// Extract and validate the JWT from the Authorization: Bearer <token> header.
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
    };

    // Insert auth context into request extensions for handlers to use
    request.extensions_mut().insert(auth_ctx);

    Ok(next.run(request).await)
}

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

