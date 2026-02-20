//! Bot API routes — create and manage bot applications, install bots to servers.
//!
//! Bot tokens use the format "Bot <token>" in the Authorization header (same
//! as Discord's bot token scheme). This file handles the developer-facing REST
//! API for managing bot applications. Bot authentication is handled by
//! `bot_auth_middleware` in `middleware.rs`.

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::bot::{BotApplication, BotServerInstall, BotToken, CreateBotRequest, UpdateBotRequest},
    snowflake,
};
use nexus_db::repository::bots;
use rand::distr::Alphanumeric;
use rand::Rng;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

/// Bot application routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Developer portal — requires user auth
        .route("/applications", post(create_application).get(list_applications))
        .route(
            "/applications/{app_id}",
            get(get_application)
                .patch(update_application)
                .delete(delete_application),
        )
        .route("/applications/{app_id}/token/reset", post(reset_token))
        // Server bot integrations
        .route(
            "/servers/{server_id}/integrations",
            get(list_server_bots).post(install_bot),
        )
        .route(
            "/servers/{server_id}/integrations/{bot_id}",
            delete(uninstall_bot),
        )
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

// ============================================================================
// Helpers
// ============================================================================

/// Generate a cryptographically random bot token (48 URL-safe chars).
fn generate_bot_token() -> String {
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}

/// Hash a bot token using SHA-256 (stored in the DB).
fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate an Ed25519 public key for webhook signature verification (hex).
fn generate_public_key() -> String {
    // For now, generate a random 32-byte value as a placeholder.
    // Full Ed25519 keypair generation would be done client-side in production.
    let key: Vec<u8> = (0..32).map(|_| rand::rng().random::<u8>()).collect();
    hex::encode(key)
}

// ============================================================================
// Developer Portal Endpoints
// ============================================================================

/// GET /api/v1/applications — List all bot applications owned by the current user.
async fn list_applications(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<Vec<BotApplication>>> {
    let bots = bots::get_bots_by_owner(&state.db.pool, auth.user_id).await?;
    Ok(Json(bots))
}

/// GET /api/v1/applications/{app_id} — Get a specific bot application.
async fn get_application(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<Uuid>,
) -> NexusResult<Json<BotApplication>> {
    let bot = bots::get_bot(&state.db.pool, app_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "application".to_string() })?;

    if bot.owner_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }

    Ok(Json(bot))
}

/// POST /api/v1/applications — Create a new bot application.
async fn create_application(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateBotRequest>,
) -> NexusResult<Json<(BotApplication, BotToken)>> {
    let app_id = snowflake::generate_id();
    let raw_token = generate_bot_token();
    let token_hash = hash_token(&raw_token);
    let public_key = generate_public_key();

    let bot = bots::create_bot(
        &state.db.pool,
        app_id,
        auth.user_id,
        &body.name,
        body.description.as_deref(),
        &token_hash,
        &public_key,
        body.is_public.unwrap_or(false),
        &body.redirect_uris.unwrap_or_default(),
        body.interactions_endpoint_url.as_deref(),
    )
    .await?;

    // Token is only returned once on creation
    Ok(Json((bot, BotToken { token: format!("Bot {raw_token}") })))
}

/// PATCH /api/v1/applications/{app_id} — Update a bot application.
async fn update_application(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<Uuid>,
    Json(body): Json<UpdateBotRequest>,
) -> NexusResult<Json<BotApplication>> {
    // Verify ownership
    let existing = bots::get_bot(&state.db.pool, app_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "application".to_string() })?;
    if existing.owner_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }

    let updated = bots::update_bot(
        &state.db.pool,
        app_id,
        body.name.as_deref(),
        body.description.as_deref(),
        body.avatar.as_deref(),
        body.is_public,
        body.redirect_uris.as_deref(),
        body.interactions_endpoint_url.as_deref(),
    )
    .await?
    .ok_or(NexusError::NotFound { resource: "application".to_string() })?;

    Ok(Json(updated))
}

/// DELETE /api/v1/applications/{app_id} — Delete a bot application.
async fn delete_application(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<Uuid>,
) -> NexusResult<axum::http::StatusCode> {
    let existing = bots::get_bot(&state.db.pool, app_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "application".to_string() })?;
    if existing.owner_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }

    bots::delete_bot(&state.db.pool, app_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// POST /api/v1/applications/{app_id}/token/reset — Regenerate the bot token.
async fn reset_token(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<Uuid>,
) -> NexusResult<Json<BotToken>> {
    let existing = bots::get_bot(&state.db.pool, app_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "application".to_string() })?;
    if existing.owner_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }

    let raw_token = generate_bot_token();
    let token_hash = hash_token(&raw_token);
    bots::update_bot_token(&state.db.pool, app_id, &token_hash).await?;

    Ok(Json(BotToken { token: format!("Bot {raw_token}") }))
}

// ============================================================================
// Server Integration Endpoints
// ============================================================================

/// GET /api/v1/servers/{server_id}/integrations — List bots in a server.
async fn list_server_bots(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<BotServerInstall>>> {
    let installs = bots::get_server_bots(&state.db.pool, server_id).await?;
    Ok(Json(installs))
}

/// POST /api/v1/servers/{server_id}/integrations — Install a bot.
#[derive(serde::Deserialize)]
struct InstallBotBody {
    bot_id: Uuid,
    permissions: Option<i64>,
    scopes: Option<Vec<String>>,
}

async fn install_bot(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<InstallBotBody>,
) -> NexusResult<Json<BotServerInstall>> {
    // Verify bot exists
    let _bot = bots::get_bot(&state.db.pool, body.bot_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "bot".to_string() })?;

    let scopes = body.scopes.unwrap_or_else(|| vec!["bot".to_string()]);
    let permissions = body.permissions.unwrap_or(0);

    let install = bots::install_bot_to_server(
        &state.db.pool,
        body.bot_id,
        server_id,
        auth.user_id,
        &scopes,
        permissions,
    )
    .await?;

    Ok(Json(install))
}

/// DELETE /api/v1/servers/{server_id}/integrations/{bot_id} — Uninstall a bot.
async fn uninstall_bot(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((server_id, bot_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<axum::http::StatusCode> {
    bots::uninstall_bot_from_server(&state.db.pool, bot_id, server_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
