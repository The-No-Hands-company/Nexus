//! Webhook routes — create, manage, and execute webhooks.
//!
//! Incoming webhook execution URL: POST /webhooks/{id}/{token}
//! (No auth required — token in URL path authenticates the request.)

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::GatewayEvent,
    models::webhook::{
        CreateIncomingWebhookRequest, CreateOutgoingWebhookRequest, ExecuteWebhookRequest,
        ModifyWebhookRequest, Webhook,
    },
    snowflake,
};
use nexus_db::repository::{channels, messages, webhooks};
use rand::distr::Alphanumeric;
use rand::Rng;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

/// Webhook routes — authenticated management + unauthenticated execution.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Authenticated management
        .route(
            "/channels/{channel_id}/webhooks",
            get(get_channel_webhooks)
                .post(create_incoming_webhook)
                .route_layer(middleware::from_fn(crate::middleware::auth_middleware)),
        )
        .route(
            "/servers/{server_id}/webhooks/outgoing",
            post(create_outgoing_webhook)
                .route_layer(middleware::from_fn(crate::middleware::auth_middleware)),
        )
        .route(
            "/webhooks/{webhook_id}",
            get(get_webhook_authed)
                .patch(modify_webhook)
                .delete(delete_webhook)
                .route_layer(middleware::from_fn(crate::middleware::auth_middleware)),
        )
        // Public execution URL — token in path, no Bearer required
        .route(
            "/webhooks/{webhook_id}/{token}",
            post(execute_webhook).get(get_webhook_public),
        )
}

/// Generate a random webhook token.
fn generate_webhook_token() -> String {
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

// ============================================================================
// Management Endpoints (require user auth)
// ============================================================================

/// GET /api/v1/channels/{channel_id}/webhooks
async fn get_channel_webhooks(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Vec<Webhook>>> {
    let hooks = webhooks::get_channel_webhooks(&state.db.pg, channel_id).await?;
    Ok(Json(hooks))
}

/// POST /api/v1/channels/{channel_id}/webhooks — Create an incoming webhook.
async fn create_incoming_webhook(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateIncomingWebhookRequest>,
) -> NexusResult<Json<Webhook>> {
    // Look up the channel to get server_id
    let channel = channels::find_by_id(&state.db.pg, channel_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "channel".to_string() })?;
    let server_id = channel.server_id.ok_or(NexusError::Validation {
        message: "Webhooks can only be created on server channels".into(),
    })?;

    let id = snowflake::generate_id();
    let token = generate_webhook_token();

    let wh = webhooks::create_incoming_webhook(
        &state.db.pg,
        id,
        server_id,
        channel_id,
        auth.user_id,
        &body.name,
        body.avatar.as_deref(),
        &token,
    )
    .await?;

    Ok(Json(wh))
}

/// POST /api/v1/servers/{server_id}/webhooks/outgoing — Create an outgoing webhook.
async fn create_outgoing_webhook(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateOutgoingWebhookRequest>,
) -> NexusResult<Json<Webhook>> {
    let id = snowflake::generate_id();

    let wh = webhooks::create_outgoing_webhook(
        &state.db.pg,
        id,
        server_id,
        auth.user_id,
        &body.name,
        &body.url,
        &body.events,
        body.avatar.as_deref(),
    )
    .await?;

    Ok(Json(wh))
}

/// GET /api/v1/webhooks/{webhook_id} — Get webhook info (with token, for owner).
async fn get_webhook_authed(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(webhook_id): Path<Uuid>,
) -> NexusResult<Json<Webhook>> {
    let wh = webhooks::get_webhook(&state.db.pg, webhook_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "webhook".to_string() })?;

    // Only creator can see full token
    if wh.creator_id != Some(auth.user_id) {
        return Ok(Json(Webhook { token: None, ..wh }));
    }

    Ok(Json(wh))
}

/// PATCH /api/v1/webhooks/{webhook_id} — Modify a webhook.
async fn modify_webhook(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(webhook_id): Path<Uuid>,
    Json(body): Json<ModifyWebhookRequest>,
) -> NexusResult<Json<Webhook>> {
    let existing = webhooks::get_webhook(&state.db.pg, webhook_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "webhook".to_string() })?;
    if existing.creator_id != Some(auth.user_id) {
        return Err(NexusError::Forbidden);
    }

    let updated = webhooks::update_webhook(
        &state.db.pg,
        webhook_id,
        body.name.as_deref(),
        body.avatar.as_deref(),
        body.channel_id,
        body.url.as_deref(),
        body.events.as_deref(),
        body.active,
    )
    .await?
    .ok_or(NexusError::NotFound { resource: "webhook".to_string() })?;

    Ok(Json(updated))
}

/// DELETE /api/v1/webhooks/{webhook_id} — Delete a webhook.
async fn delete_webhook(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(webhook_id): Path<Uuid>,
) -> NexusResult<axum::http::StatusCode> {
    let existing = webhooks::get_webhook(&state.db.pg, webhook_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "webhook".to_string() })?;
    if existing.creator_id != Some(auth.user_id) {
        return Err(NexusError::Forbidden);
    }

    webhooks::delete_webhook(&state.db.pg, webhook_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ============================================================================
// Public Execution Endpoints (no user auth — token in URL)
// ============================================================================

/// GET /api/v1/webhooks/{webhook_id}/{token} — Get webhook info (public, no owner token).
async fn get_webhook_public(
    State(state): State<Arc<AppState>>,
    Path((webhook_id, token)): Path<(Uuid, String)>,
) -> NexusResult<Json<Webhook>> {
    let wh = webhooks::get_webhook_by_token(&state.db.pg, webhook_id, &token)
        .await?
        .ok_or(NexusError::NotFound { resource: "webhook".to_string() })?;
    Ok(Json(wh))
}

/// POST /api/v1/webhooks/{webhook_id}/{token} — Execute a webhook (post a message).
async fn execute_webhook(
    State(state): State<Arc<AppState>>,
    Path((webhook_id, token)): Path<(Uuid, String)>,
    Json(body): Json<ExecuteWebhookRequest>,
) -> NexusResult<axum::http::StatusCode> {
    // Validate token
    let wh = webhooks::get_webhook_by_token(&state.db.pg, webhook_id, &token)
        .await?
        .ok_or(NexusError::NotFound { resource: "webhook".to_string() })?;

    let channel_id = wh.channel_id.ok_or(NexusError::Validation {
        message: "Webhook has no target channel".into(),
    })?;

    let content = body.content.unwrap_or_default();
    if content.is_empty() && body.embeds.as_ref().map_or(true, |e| e.is_empty()) {
        return Err(NexusError::Validation {
            message: "content or embeds must be provided".into(),
        });
    }

    // Create the message as a "webhook" author
    let msg_id = snowflake::generate_id();
    let display_name = body.username.as_deref().unwrap_or(&wh.name);

    // Build a stub message record — in production this would go through the
    // full message creation pipeline including thread resolution, embeds, etc.
    let message = messages::create_message(
        &state.db.pg,
        msg_id,
        channel_id,
        // Use webhook UUID as pseudo user_id
        webhook_id,
        &content,
        0,    // message_type: normal
        None, // reference_message_id
        None, // reference_channel_id
        &[],  // mentions
        &[],  // mention_roles
        false, // mention_everyone
    )
    .await?;

    // Broadcast MESSAGE_CREATE via the gateway
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: nexus_common::gateway_event::event_types::MESSAGE_CREATE.to_string(),
        data: serde_json::json!({
            "message_id": message.id,
            "channel_id": channel_id,
            "webhook_id": webhook_id,
            "username": display_name,
            "avatar_url": body.avatar_url,
            "content": &content,
        }),
        server_id: wh.server_id,
        channel_id: Some(channel_id),
        user_id: None,
    });

    Ok(axum::http::StatusCode::NO_CONTENT)
}
