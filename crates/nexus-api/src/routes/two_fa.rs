//! Two-Factor Authentication (TOTP/RFC-6238) routes.
//!
//! Endpoints:
//!   POST /auth/2fa/setup              — generate TOTP secret + backup codes
//!   POST /auth/2fa/enable             — confirm first TOTP code, activate 2FA
//!   POST /auth/2fa/disable            — verify password + code, remove 2FA
//!   POST /auth/2fa/verify             — complete MFA login challenge
//!   GET  /auth/2fa/backup-codes       — count of unused backup codes
//!   POST /auth/2fa/backup-codes/regenerate — generate a fresh set of 8 codes

use axum::http::HeaderMap;
use axum::{
    extract::{Extension, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_db::repository::{sessions, two_fa, users};
use rand::Rng;
use rand::distr::{Alphanumeric, Distribution};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use totp_rs::{Algorithm, Secret, TOTP};
use chrono::{Duration, Utc};
use crate::{auth, middleware::{AuthContext, check_rate_limit_with_fallback, extract_client_ip}, AppState};

// ── Constants ────────────────────────────────────────────────────────────────

const ISSUER: &str = "Nexus";
const BACKUP_CODE_COUNT: usize = 10; // 10 single-use codes
const BACKUP_CODE_LEN: usize = 12;  // 12 chars × 62 alphabet = ~71 bits entropy

// ── Router ───────────────────────────────────────────────────────────────────

/// Public (unauthenticated) 2FA routes — only the MFA challenge verify.
pub fn public_router() -> Router<Arc<AppState>> {
    Router::new().route("/auth/2fa/verify", post(verify_mfa))
}

/// Authenticated 2FA management routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/2fa/setup", post(setup))
        .route("/auth/2fa/enable", post(enable))
        .route("/auth/2fa/disable", post(disable))
        .route("/auth/2fa/backup-codes", get(backup_code_count))
        .route("/auth/2fa/backup-codes/regenerate", post(regenerate_backup_codes))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build a `TOTP` instance from a base32-encoded secret string.
fn totp_from_secret(secret_b32: &str, username: &str) -> Result<TOTP, NexusError> {
    let secret_bytes = Secret::Encoded(secret_b32.to_string())
        .to_bytes()
        .map_err(|e| NexusError::Internal(anyhow::anyhow!("TOTP secret decode error: {e}")))?;

    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some(ISSUER.to_string()),
        username.to_string(),
    )
    .map_err(|e| NexusError::Internal(anyhow::anyhow!("TOTP init error: {e}")))
}

/// Generate `BACKUP_CODE_COUNT` random alphanumeric backup codes.
fn generate_raw_backup_codes() -> Vec<String> {
    // Use OsRng (CSPRNG) — never the thread-local SmallRng which is not
    // cryptographically secure. 10 codes × 12 alphanumeric chars ≈ 71 bits
    // entropy per code — exceeds NIST SP 800-63B single-use token guidance.
    use rand::distr::Alphanumeric;
    use rand::distr::Distribution;
    use rand::rngs::OsRng;
    let mut rng = OsRng;
    (0..BACKUP_CODE_COUNT)
        .map(|_| {
            Alphanumeric
                .sample_iter(&mut rng)
                .take(BACKUP_CODE_LEN)
                .map(char::from)
                .collect::<String>()
                .to_uppercase()
        })
        .collect()
}

// ── Setup ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct SetupResponse {
    /// Base32-encoded TOTP secret (for manual entry into authenticator apps).
    secret: String,
    /// otpauth:// URI for QR code generation on the client.
    provisioning_uri: String,
    /// 8 single-use backup codes shown **once**.  Store them safely.
    backup_codes: Vec<String>,
}

/// POST /api/v1/auth/2fa/setup
///
/// Generate a TOTP secret and backup codes.  Does NOT enable 2FA yet.
/// Call `POST /auth/2fa/enable` after verifying the first code.
async fn setup(
    Extension(auth_ctx): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<SetupResponse>> {
    // Rate limiting: 5 TOTP setup attempts per user per hour (prevents enumeration)
    let user = users::find_by_id(&state.db.pool, auth_ctx.user_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "User".into() })?;

    // Generate a fresh base32-encoded TOTP secret.
    // `Secret::generate_secret()` (from `gen_secret` feature) returns `Secret::Encoded(b32)`.
    let raw_secret = Secret::generate_secret();
    let secret_b32 = match raw_secret {
        Secret::Encoded(s) => s,
        // Defensive: the gen_secret feature always returns Encoded; this should be unreachable.
        Secret::Raw(_) => {
            return Err(NexusError::Internal(anyhow::anyhow!(
                "TOTP generate_secret returned Raw variant; expected Encoded"
            )));
        }
    };

    // Save the pending secret (totp_enabled stays false until confirmed)
    two_fa::set_totp_secret(&state.db.pool, user.id, &secret_b32)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    // Build provisioning URI
    let totp = totp_from_secret(&secret_b32, &user.username)?;
    let provisioning_uri = totp.get_url();

    // Generate and persist backup codes
    let raw_codes = generate_raw_backup_codes();
    let hashes: Vec<String> = raw_codes.iter().map(|c| two_fa::hash_backup_code(c)).collect();
    two_fa::replace_backup_codes(&state.db.pool, user.id, &hashes)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    tracing::info!(user_id = %user.id, "TOTP setup initiated");

    Ok(Json(SetupResponse {
        secret: secret_b32,
        provisioning_uri,
        backup_codes: raw_codes,
    }))
}

// ── Enable ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct EnableBody {
    /// TOTP code from the authenticator app — confirms the user scanned the QR correctly.
    totp_code: String,
}

/// POST /api/v1/auth/2fa/enable
///
/// Activate TOTP after verifying the first code from the authenticator app.
async fn enable(
    Extension(auth_ctx): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<EnableBody>,
) -> NexusResult<()> {
    // Rate limiting: 10 TOTP enable attempts per user per 10 minutes (prevents brute force on verification code)
    let ip = extract_client_ip(&headers);
    check_rate_limit_with_fallback(
        state.db.redis.as_ref(),
        format!("rl:2fa:enable:user:{}", auth_ctx.user_id),
        10,
        600,
    ).await?;
    check_rate_limit_with_fallback(
        state.db.redis.as_ref(),
        format!("rl:2fa:enable:ip:{ip}"),
        20,
        600,
    ).await?;

    let user = users::find_by_id(&state.db.pool, auth_ctx.user_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "User".into() })?;

    if user.totp_enabled {
        return Err(NexusError::AlreadyExists { resource: "TOTP 2FA".into() });
    }

    let secret_b32 = two_fa::get_totp_secret(&state.db.pool, user.id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or_else(|| NexusError::Validation { message: "No TOTP setup in progress. Call /auth/2fa/setup first.".into() })?;

    let totp = totp_from_secret(&secret_b32, &user.username)?;
    let valid = totp
        .check_current(&body.totp_code)
        .map_err(|e| NexusError::Internal(anyhow::anyhow!("TOTP check error: {e}")))?;

    if !valid {
        return Err(NexusError::InvalidCredentials);
    }

    two_fa::enable_totp(&state.db.pool, user.id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    tracing::info!(user_id = %user.id, "TOTP 2FA enabled");
    Ok(())
}

// ── Disable ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct DisableBody {
    password: String,
    /// Current TOTP code from authenticator app (or a backup code).
    totp_code: String,
}

/// POST /api/v1/auth/2fa/disable
///
/// Disable TOTP after verifying the current password and a valid TOTP code.
async fn disable(
    Extension(auth_ctx): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DisableBody>,
) -> NexusResult<()> {
    // Rate limiting: 10 TOTP disable attempts per user per 10 minutes
    let ip = extract_client_ip(&headers);
    check_rate_limit_with_fallback(
        state.db.redis.as_ref(),
        format!("rl:2fa:disable:user:{}", auth_ctx.user_id),
        10,
        600,
    ).await?;
    check_rate_limit_with_fallback(
        state.db.redis.as_ref(),
        format!("rl:2fa:disable:ip:{ip}"),
        20,
        600,
    ).await?;

    let user = users::find_by_id(&state.db.pool, auth_ctx.user_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "User".into() })?;

    if !user.totp_enabled {
        return Err(NexusError::Validation { message: "2FA is not enabled on this account".into() });
    }

    // Verify password
    let valid = auth::verify_password(&body.password, &user.password_hash)
        .map_err(|_| NexusError::InvalidCredentials)?;
    if !valid {
        return Err(NexusError::InvalidCredentials);
    }

    // Verify TOTP code or backup code
    let secret_b32 = two_fa::get_totp_secret(&state.db.pool, user.id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or_else(|| NexusError::Internal(anyhow::anyhow!("TOTP enabled but no secret stored")))?;

    let totp = totp_from_secret(&secret_b32, &user.username)?;
    let totp_valid = totp
        .check_current(&body.totp_code)
        .map_err(|e| NexusError::Internal(anyhow::anyhow!("TOTP check error: {e}")))?;

    // Accept TOTP code or backup code
    let backup_valid = if !totp_valid {
        two_fa::consume_backup_code(&state.db.pool, user.id, &body.totp_code)
            .await
            .map_err(|e| NexusError::Internal(e.into()))?
    } else {
        false
    };

    if !totp_valid && !backup_valid {
        return Err(NexusError::InvalidCredentials);
    }

    two_fa::disable_totp(&state.db.pool, user.id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    tracing::info!(user_id = %user.id, "TOTP 2FA disabled");
    Ok(())
}

// ── Verify (complete MFA login challenge) ────────────────────────────────────

#[derive(Deserialize)]
struct VerifyMfaBody {
    /// The `mfa_token` returned by the login endpoint.
    mfa_token: String,
    /// 6-digit TOTP code from the authenticator app, or a backup code.
    code: String,
    /// Optional device hint stored in the session row.
    device_info: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct AuthResponse {
    #[serde(flatten)]
    pub tokens: auth::TokenPair,
}

/// POST /api/v1/auth/2fa/verify
///
/// Exchange an MFA challenge token + TOTP code for a full token pair.
async fn verify_mfa(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<VerifyMfaBody>,
) -> NexusResult<Json<AuthResponse>> {
    let config = nexus_common::config::get();

    // Validate the challenge token first to get the user_id
    let claims = auth::validate_token(&body.mfa_token, &config.auth.jwt_secret)
        .map_err(|_| NexusError::InvalidToken)?;
    if claims.token_type != "mfa_challenge" {
        return Err(NexusError::InvalidToken);
    }

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| NexusError::InvalidToken)?;

    // CRITICAL: Rate limiting on MFA verification to prevent brute force attacks
    // 5 attempts per user per 5 minutes (stricter than other endpoints)
    let ip = extract_client_ip(&headers);
    check_rate_limit_with_fallback(
        state.db.redis.as_ref(),
        format!("rl:2fa:verify:user:{}", user_id),
        5,
        300,
    ).await?;
    check_rate_limit_with_fallback(
        state.db.redis.as_ref(),
        format!("rl:2fa:verify:ip:{ip}"),
        10,
        300,
    ).await?;

    let user = users::find_by_id(&state.db.pool, user_id)
        .await?
        .ok_or(NexusError::InvalidToken)?;

    if !user.totp_enabled {
        return Err(NexusError::InvalidToken);
    }

    let secret_b32 = two_fa::get_totp_secret(&state.db.pool, user.id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or_else(|| NexusError::Internal(anyhow::anyhow!("TOTP enabled but no secret found")))?;

    let totp = totp_from_secret(&secret_b32, &user.username)?;
    let totp_valid = totp
        .check_current(&body.code)
        .map_err(|e| NexusError::Internal(anyhow::anyhow!("TOTP check error: {e}")))?;

    let backup_valid = if !totp_valid {
        two_fa::consume_backup_code(&state.db.pool, user.id, &body.code)
            .await
            .map_err(|e| NexusError::Internal(e.into()))?
    } else {
        false
    };

    if !totp_valid && !backup_valid {
        return Err(NexusError::InvalidCredentials);
    }

    // Issue full token pair with two_fa_verified = true
    let email_verified = user.email.is_none()
        || user.flags & nexus_common::models::user::user_flags::EMAIL_VERIFIED != 0;
    let tokens = auth::generate_token_pair(
        user.id,
        &user.username,
        &config.auth.jwt_secret,
        config.auth.access_token_ttl_secs,
        config.auth.refresh_token_ttl_secs,
        true, // two_fa_verified
        email_verified,
    )
    .map_err(|e| NexusError::Internal(e.into()))?;

    // Persist session
    let expires_at = Utc::now() + Duration::seconds(config.auth.refresh_token_ttl_secs as i64);
    sessions::create_session(
        &state.db.pool,
        tokens.session_id,
        user.id,
        body.device_info.as_deref(),
        None,
        None,
        expires_at,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    tracing::info!(user_id = %user.id, "MFA verification successful");
    Ok(Json(AuthResponse { tokens }))
}

// ── Backup code management ───────────────────────────────────────────────────

#[derive(Serialize)]
struct BackupCodeCountResponse {
    remaining: i64,
}

/// GET /api/v1/auth/2fa/backup-codes
///
/// Return the count of unused backup codes.
async fn backup_code_count(
    Extension(auth_ctx): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<BackupCodeCountResponse>> {
    let remaining = two_fa::count_unused_backup_codes(&state.db.pool, auth_ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(BackupCodeCountResponse { remaining }))
}

#[derive(Deserialize)]
struct RegenerateBody {
    /// Current TOTP code — required to regenerate codes (prevents abuse).
    totp_code: String,
}

#[derive(Serialize)]
struct RegenerateResponse {
    /// 8 fresh backup codes shown one time only.
    backup_codes: Vec<String>,
}

/// POST /api/v1/auth/2fa/backup-codes/regenerate
///
/// Generate a fresh set of 8 backup codes.
/// Requires a valid current TOTP code to prevent abuse.
async fn regenerate_backup_codes(
    Extension(auth_ctx): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RegenerateBody>,
) -> NexusResult<Json<RegenerateResponse>> {
    // Rate limiting: 3 backup code regenerations per user per hour
    let ip = extract_client_ip(&headers);
    check_rate_limit_with_fallback(
        state.db.redis.as_ref(),
        format!("rl:2fa:regen:user:{}", auth_ctx.user_id),
        3,
        3600,
    ).await?;
    check_rate_limit_with_fallback(
        state.db.redis.as_ref(),
        format!("rl:2fa:regen:ip:{ip}"),
        5,
        3600,
    ).await?;

    let user = users::find_by_id(&state.db.pool, auth_ctx.user_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "User".into() })?;

    if !user.totp_enabled {
        return Err(NexusError::Validation { message: "2FA is not enabled on this account".into() });
    }

    let secret_b32 = two_fa::get_totp_secret(&state.db.pool, user.id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or_else(|| NexusError::Internal(anyhow::anyhow!("TOTP enabled but no secret")))?;

    let totp = totp_from_secret(&secret_b32, &user.username)?;
    let valid = totp
        .check_current(&body.totp_code)
        .map_err(|e| NexusError::Internal(anyhow::anyhow!("TOTP check error: {e}")))?;
    if !valid {
        return Err(NexusError::InvalidCredentials);
    }

    let raw_codes = generate_raw_backup_codes();
    let hashes: Vec<String> = raw_codes.iter().map(|c| two_fa::hash_backup_code(c)).collect();
    two_fa::replace_backup_codes(&state.db.pool, user.id, &hashes)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    tracing::info!(user_id = %user.id, "Backup codes regenerated");
    Ok(Json(RegenerateResponse { backup_codes: raw_codes }))
}
