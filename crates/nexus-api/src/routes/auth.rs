//! Authentication routes — register, login, refresh, logout.
//!
//! Privacy-first: No phone number. No ID. No age verification.
//! Just a username and password. Email is optional (only for password reset).

use axum::{extract::State, routing::post, Json, Router};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::user::{CreateUserRequest, LoginRequest, UserResponse},
    snowflake,
    validation::validate_request,
};
use nexus_db::repository::users;
use serde::Serialize;
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

#[derive(Serialize)]
struct AuthResponse {
    user: UserResponse,
    #[serde(flatten)]
    tokens: TokenPair,
}

/// POST /api/v1/auth/register
///
/// Create a new account. Returns user profile + JWT tokens.
/// No email required. No phone. No ID. Just pick a username and password.
async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateUserRequest>,
) -> NexusResult<Json<AuthResponse>> {
    validate_request(&body)?;

    // Check username availability
    if users::find_by_username(&state.db.pool, &body.username)
        .await?
        .is_some()
    {
        return Err(NexusError::AlreadyExists {
            resource: "Username".into(),
        });
    }

    // Check email availability (if provided)
    if let Some(ref email) = body.email {
        if users::find_by_email(&state.db.pool, email).await?.is_some() {
            return Err(NexusError::AlreadyExists {
                resource: "Email".into(),
            });
        }
    }

    // Hash password with Argon2id
    let password_hash =
        auth::hash_password(&body.password).map_err(|e| NexusError::Internal(anyhow::anyhow!("{e}")))?;

    // Generate user ID
    let user_id = snowflake::generate_id();

    // Create user
    let user = users::create_user(
        &state.db.pool,
        user_id,
        &body.username,
        body.email.as_deref(),
        &password_hash,
    )
    .await?;

    // Generate tokens
    let config = nexus_common::config::get();
    let tokens = auth::generate_token_pair(
        user.id,
        &user.username,
        &config.auth.jwt_secret,
        config.auth.access_token_ttl_secs,
        config.auth.refresh_token_ttl_secs,
    )
    .map_err(|e| NexusError::Internal(e.into()))?;

    tracing::info!(user_id = %user.id, username = %user.username, "New user registered");

    Ok(Json(AuthResponse {
        user: user.into(),
        tokens,
    }))
}

/// POST /api/v1/auth/login
///
/// Authenticate with username + password. Returns JWT tokens.
async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> NexusResult<Json<AuthResponse>> {
    validate_request(&body)?;

    // Find user
    let user = users::find_by_username(&state.db.pool, &body.username)
        .await?
        .ok_or(NexusError::InvalidCredentials)?;

    // Verify password
    let valid =
        auth::verify_password(&body.password, &user.password_hash).map_err(|_| NexusError::InvalidCredentials)?;

    if !valid {
        return Err(NexusError::InvalidCredentials);
    }

    // Check if account is disabled/suspended
    if user.flags & nexus_common::models::user::user_flags::DISABLED != 0 {
        return Err(NexusError::Forbidden);
    }
    if user.flags & nexus_common::models::user::user_flags::SUSPENDED != 0 {
        return Err(NexusError::Forbidden);
    }

    // Generate tokens
    let config = nexus_common::config::get();
    let tokens = auth::generate_token_pair(
        user.id,
        &user.username,
        &config.auth.jwt_secret,
        config.auth.access_token_ttl_secs,
        config.auth.refresh_token_ttl_secs,
    )
    .map_err(|e| NexusError::Internal(e.into()))?;

    tracing::info!(user_id = %user.id, "User logged in");

    Ok(Json(AuthResponse {
        user: user.into(),
        tokens,
    }))
}

/// POST /api/v1/auth/refresh
///
/// Exchange a refresh token for a new token pair.
async fn refresh_token(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RefreshRequest>,
) -> NexusResult<Json<TokenPair>> {
    let config = nexus_common::config::get();

    // Validate the refresh token
    let claims = auth::validate_token(&body.refresh_token, &config.auth.jwt_secret)
        .map_err(|_| NexusError::InvalidToken)?;

    if claims.token_type != "refresh" {
        return Err(NexusError::InvalidToken);
    }

    let user_id: uuid::Uuid = claims
        .sub
        .parse()
        .map_err(|_| NexusError::InvalidToken)?;

    // Verify user still exists and isn't disabled
    let user = users::find_by_id(&state.db.pool, user_id)
        .await?
        .ok_or(NexusError::InvalidToken)?;

    // Generate new token pair
    let tokens = auth::generate_token_pair(
        user.id,
        &user.username,
        &config.auth.jwt_secret,
        config.auth.access_token_ttl_secs,
        config.auth.refresh_token_ttl_secs,
    )
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(tokens))
}

#[derive(serde::Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}
