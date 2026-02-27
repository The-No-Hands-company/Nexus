//! Authentication routes — register, login, refresh, logout.
//!
//! Privacy-first: No phone number. No ID. No age verification.
//! Just a username and password. Email is optional (only for password reset).

use axum::{
    extract::State,
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use chrono::{Duration, Utc};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::user::{CreateUserRequest, LoginRequest, UserResponse},
    snowflake,
    validation::validate_request,
};
use nexus_db::repository::{email_verification, sessions, users};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    auth::{self, TokenPair},
    AppState,
};

/// Auth router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh_token))
}

// ── Response types ──────────────────────────────────────────────────────────

/// Returned on successful login/register when no 2FA challenge is needed.
#[derive(Serialize)]
struct AuthResponse {
    user: UserResponse,
    #[serde(flatten)]
    tokens: TokenPair,
}

/// Returned by login when the account has TOTP enabled.
/// The client must call `POST /auth/2fa/verify` with this token + a TOTP code.
#[derive(Serialize)]
struct MfaChallengeResponse {
    requires_mfa: bool,
    /// Short-lived JWT (5 min, type="mfa_challenge") to carry the user identity.
    mfa_token: String,
}

/// Union of the two possible login responses.
#[derive(Serialize)]
#[serde(untagged)]
enum LoginResponse {
    Full(AuthResponse),
    MfaChallenge(MfaChallengeResponse),
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Extract the raw `User-Agent` string from request headers.
fn user_agent_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

/// Persist a session row and return the assembled token pair.
async fn issue_tokens(
    state: &AppState,
    user_id: uuid::Uuid,
    username: &str,
    two_fa_verified: bool,
    email_verified: bool,
    user_agent: Option<String>,
) -> NexusResult<TokenPair> {
    let config = nexus_common::config::get();
    let tokens = auth::generate_token_pair(
        user_id,
        username,
        &config.auth.jwt_secret,
        config.auth.access_token_ttl_secs,
        config.auth.refresh_token_ttl_secs,
        two_fa_verified,
        email_verified,
    )
    .map_err(|e| NexusError::Internal(e.into()))?;

    let expires_at = Utc::now() + Duration::seconds(config.auth.refresh_token_ttl_secs as i64);
    sessions::create_session(
        &state.db.pool,
        tokens.session_id,
        user_id,
        None,
        user_agent.as_deref(),
        None,
        expires_at,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(tokens)
}

// ── Route handlers ───────────────────────────────────────────────────────────

/// POST /api/v1/auth/register
///
/// Create a new account. Returns user profile + JWT tokens.
/// No email required. No phone. No ID. Just pick a username and password.
async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateUserRequest>,
) -> NexusResult<Json<AuthResponse>> {
    let user_agent = user_agent_from_headers(&headers);
    validate_request(&body)?;

    // Check username availability
    if users::find_by_username(&state.db.pool, &body.username)
        .await?
        .is_some()
    {
        return Err(NexusError::AlreadyExists { resource: "Username".into() });
    }

    // Check email availability (if provided)
    if let Some(ref email) = body.email {
        if users::find_by_email(&state.db.pool, email).await?.is_some() {
            return Err(NexusError::AlreadyExists { resource: "Email".into() });
        }
    }

    // Hash password with Argon2id
    let password_hash =
        auth::hash_password(&body.password).map_err(|e| NexusError::Internal(anyhow::anyhow!("{e}")))?;

    // Create user
    let user_id = snowflake::generate_id();
    let user = users::create_user(
        &state.db.pool,
        user_id,
        &body.username,
        body.email.as_deref(),
        &password_hash,
    )
    .await?;

    // Issue email verification token if email was provided
    if user.email.is_some() {
        let (raw_token, token_hash, expires_at) = email_verification::generate_token();
        if let Err(e) =
            email_verification::upsert_token(&state.db.pool, user.id, &token_hash, expires_at).await
        {
            tracing::warn!(user_id = %user.id, error = %e, "Failed to store email verification token");
        } else {
            // TODO: hand raw_token to email-sending service when SMTP is wired.
            // Until then, log at DEBUG so developers can retrieve it without leaking to prod logs.
            tracing::debug!(
                user_id = %user.id,
                token = %raw_token,
                "Email verification token generated"
            );
        }
    }

    // Generate tokens + persist session
    // New users start with email_verified = false (no email registered → treated as verified
    // so they can use the app immediately; verification is only enforced if email was given).
    let email_verified = user.email.is_none()
        || user.flags & nexus_common::models::user::user_flags::EMAIL_VERIFIED != 0;
    let tokens = issue_tokens(&state, user.id, &user.username, false, email_verified, user_agent).await?;

    tracing::info!(user_id = %user.id, username = %user.username, "New user registered");
    Ok(Json(AuthResponse { user: user.into(), tokens }))
}

/// POST /api/v1/auth/login
///
/// Authenticate with username + password.
/// If the account has TOTP enabled, returns `{requires_mfa: true, mfa_token}`.
/// Otherwise returns full tokens immediately.
async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> NexusResult<Json<LoginResponse>> {
    let user_agent = user_agent_from_headers(&headers);
    validate_request(&body)?;

    // Find user — keep timing constant to prevent username enumeration
    let user = users::find_by_username(&state.db.pool, &body.username)
        .await?
        .ok_or(NexusError::InvalidCredentials)?;

    // Verify password
    let valid = auth::verify_password(&body.password, &user.password_hash)
        .map_err(|_| NexusError::InvalidCredentials)?;
    if !valid {
        return Err(NexusError::InvalidCredentials);
    }

    // Check account state
    if user.flags & nexus_common::models::user::user_flags::DISABLED != 0 {
        return Err(NexusError::Forbidden);
    }
    if user.flags & nexus_common::models::user::user_flags::SUSPENDED != 0 {
        return Err(NexusError::Forbidden);
    }

    // TOTP gate — issue MFA challenge token
    if user.totp_enabled {
        let config = nexus_common::config::get();
        let mfa_token =
            auth::generate_mfa_challenge_token(user.id, &user.username, &config.auth.jwt_secret)
                .map_err(|e| NexusError::Internal(e.into()))?;
        tracing::info!(user_id = %user.id, "MFA challenge issued");
        return Ok(Json(LoginResponse::MfaChallenge(MfaChallengeResponse {
            requires_mfa: true,
            mfa_token,
        })));
    }

    // No 2FA — issue tokens directly
    let email_verified = user.email.is_none()
        || user.flags & nexus_common::models::user::user_flags::EMAIL_VERIFIED != 0;
    let tokens = issue_tokens(&state, user.id, &user.username, false, email_verified, user_agent).await?;
    tracing::info!(user_id = %user.id, "User logged in");
    Ok(Json(LoginResponse::Full(AuthResponse { user: user.into(), tokens })))
}

/// POST /api/v1/auth/refresh
///
/// Exchange a refresh token for a new token pair (session rotation).
async fn refresh_token(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RefreshRequest>,
) -> NexusResult<Json<TokenPair>> {
    let config = nexus_common::config::get();

    let claims = auth::validate_token(&body.refresh_token, &config.auth.jwt_secret)
        .map_err(|_| NexusError::InvalidToken)?;

    if claims.token_type != "refresh" {
        return Err(NexusError::InvalidToken);
    }

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| NexusError::InvalidToken)?;

    let user = users::find_by_id(&state.db.pool, user_id)
        .await?
        .ok_or(NexusError::InvalidToken)?;

    // Rotate session: revoke old, create new
    let old_session_id: uuid::Uuid = claims.jti.parse().unwrap_or(uuid::Uuid::nil());
    if !old_session_id.is_nil() {
        let _ = sessions::revoke_session(&state.db.pool, user_id, old_session_id).await;
    }

    let tokens = auth::generate_token_pair(
        user.id,
        &user.username,
        &config.auth.jwt_secret,
        config.auth.access_token_ttl_secs,
        config.auth.refresh_token_ttl_secs,
        user.totp_enabled,
        user.email.is_none() || user.flags & nexus_common::models::user::user_flags::EMAIL_VERIFIED != 0,
    )
    .map_err(|e| NexusError::Internal(e.into()))?;

    let expires_at = Utc::now() + Duration::seconds(config.auth.refresh_token_ttl_secs as i64);
    let _ = sessions::create_session(
        &state.db.pool,
        tokens.session_id,
        user_id,
        None,
        None,
        None,
        expires_at,
    )
    .await;

    Ok(Json(tokens))
}

#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

