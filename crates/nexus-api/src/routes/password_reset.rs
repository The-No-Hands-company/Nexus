//! Password reset routes.
//!
//! Endpoints:
//!   POST /auth/forgot-password  — request a password reset email
//!   POST /auth/reset-password   — consume one-time token and set new password

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_db::repository::{password_reset, users};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{auth, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/reset-password", post(reset_password))
}

#[derive(Debug, Deserialize)]
struct ForgotPasswordBody {
    email: String,
}

#[derive(Debug, Deserialize)]
struct ResetPasswordBody {
    token: String,
    new_password: String,
}

#[derive(Debug, Serialize)]
struct GenericResponse {
    ok: bool,
    message: String,
}

async fn forgot_password(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ForgotPasswordBody>,
) -> NexusResult<Json<GenericResponse>> {
    let generic_response = Json(GenericResponse {
        ok: true,
        message: "If an account exists for this email, a reset link has been sent.".into(),
    });

    let email = body.email.trim();
    if email.is_empty() || email.len() > 255 {
        return Ok(generic_response);
    }

    let user = match users::find_by_email(&state.db.pool, email).await {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(error = %e, "forgot-password lookup failed");
            return Ok(generic_response);
        }
    };

    let Some(user) = user else {
        return Ok(generic_response);
    };

    // Basic flood protection: one valid token at a time per user.
    let has_pending = password_reset::has_pending_token(&state.db.pool, user.id)
        .await
        .unwrap_or(false);
    if has_pending {
        return Ok(generic_response);
    }

    let (raw_token, token_hash, expires_at) = password_reset::generate_token();
    if let Err(e) = password_reset::upsert_token(&state.db.pool, user.id, &token_hash, expires_at).await {
        tracing::warn!(error = %e, user_id = %user.id, "failed to upsert password reset token");
        return Ok(generic_response);
    }

    if let Some(ref email_addr) = user.email {
        if let Err(e) = state
            .email
            .send_password_reset_email(email_addr, &user.username, &raw_token)
            .await
        {
            tracing::warn!(error = %e, user_id = %user.id, "failed to send password reset email");
        }
    }

    tracing::info!(user_id = %user.id, "password reset requested");
    Ok(generic_response)
}

async fn reset_password(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ResetPasswordBody>,
) -> NexusResult<Json<GenericResponse>> {
    let token = body.token.trim();
    if token.is_empty() {
        return Err(NexusError::Validation {
            message: "token is required".into(),
        });
    }

    if body.new_password.len() < 8 || body.new_password.len() > 128 {
        return Err(NexusError::Validation {
            message: "new_password must be 8-128 characters".into(),
        });
    }

    let user_id = password_reset::consume_token(&state.db.pool, token)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or(NexusError::InvalidToken)?;

    let new_hash = auth::hash_password(&body.new_password)
        .map_err(|e| NexusError::Internal(anyhow::anyhow!(e.to_string())))?;

    users::update_password_hash(&state.db.pool, user_id, &new_hash)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    // Revoke all refresh sessions so old logins can't continue.
    let _ = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1::uuid")
        .bind(user_id.to_string())
        .execute(&state.db.pool)
        .await;

    tracing::info!(user_id = %user_id, "password reset completed");

    Ok(Json(GenericResponse {
        ok: true,
        message: "Password has been reset. Please log in again.".into(),
    }))
}
