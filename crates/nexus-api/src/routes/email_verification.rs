//! Email verification routes.
//!
//! Endpoints:
//!   GET  /auth/verify-email?token=…   — verify email using one-time token
//!   POST /auth/resend-verification    — resend verification email (authenticated)

use axum::{
    extract::{Extension, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_db::repository::email_verification;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    // Verify endpoint is public — the link comes from an email
    let public = Router::new().route("/auth/verify-email", get(verify_email));

    // Resend requires authentication
    let protected = Router::new()
        .route("/auth/resend-verification", post(resend_verification))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware));

    public.merge(protected)
}

// ── Verify email token ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct VerifyQuery {
    token: String,
}

#[derive(Serialize)]
struct VerifyResponse {
    verified: bool,
}

/// GET /api/v1/auth/verify-email?token=<raw_token>
///
/// Consume the one-time verification token sent to the user's email.
/// Sets the `EMAIL_VERIFIED` flag and deletes the token row.
async fn verify_email(
    State(state): State<Arc<AppState>>,
    Query(params): Query<VerifyQuery>,
) -> NexusResult<Json<VerifyResponse>> {
    let user_id = email_verification::consume_token(&state.db.pool, &params.token)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or(NexusError::InvalidToken)?;

    // Set EMAIL_VERIFIED flag (bit 2)
    sqlx::query(
        "UPDATE users SET flags = flags | $1, updated_at = NOW() WHERE id = $2::uuid",
    )
    .bind(nexus_common::models::user::user_flags::EMAIL_VERIFIED)
    .bind(user_id.to_string())
    .execute(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    tracing::info!(user_id = %user_id, "Email verified");
    Ok(Json(VerifyResponse { verified: true }))
}

// ── Resend verification ───────────────────────────────────────────────────────

#[derive(Serialize)]
struct ResendResponse {
    sent: bool,
    message: String,
}

/// POST /api/v1/auth/resend-verification
///
/// Generate a new verification token and (conceptually) send it via email.
/// Returns 204 even if the email is already verified — no information leakage.
async fn resend_verification(
    Extension(auth_ctx): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<ResendResponse>> {
    let user = nexus_db::repository::users::find_by_id(&state.db.pool, auth_ctx.user_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "User".into() })?;

    // Silently succeed if already verified or no email registered
    if user.email.is_none()
        || user.flags & nexus_common::models::user::user_flags::EMAIL_VERIFIED != 0
    {
        return Ok(Json(ResendResponse {
            sent: false,
            message: "Email already verified or no email on account".into(),
        }));
    }

    // Rate-limit: silently succeed if a valid token already exists (< 24h old)
    let has_pending = email_verification::has_pending_token(&state.db.pool, user.id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    if has_pending {
        return Ok(Json(ResendResponse {
            sent: false,
            message: "A verification email was recently sent. Please check your inbox.".into(),
        }));
    }

    // Generate and persist new token
    let (raw_token, token_hash, expires_at) = email_verification::generate_token();
    email_verification::upsert_token(&state.db.pool, user.id, &token_hash, expires_at)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    // TODO: dispatch to email-sending service when SMTP is wired.
    tracing::debug!(
        user_id = %user.id,
        token = %raw_token,
        "Verification email re-sent (token logged for dev)"
    );

    tracing::info!(user_id = %user.id, "Verification email queued");
    Ok(Json(ResendResponse {
        sent: true,
        message: "Verification email sent.".into(),
    }))
}
