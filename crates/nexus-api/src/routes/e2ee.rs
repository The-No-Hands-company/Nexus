//! Encrypted DM & channel messaging routes.
//!
//! POST   /channels/:id/encrypted-messages          — Send an encrypted message
//! GET    /channels/:id/encrypted-messages          — Fetch encrypted history
//! GET    /channels/:id/encrypted-messages/:msg_id  — Single message
//! PUT    /channels/:id/e2ee                        — Enable E2EE on a channel
//! GET    /channels/:id/e2ee                        — Get channel E2EE config

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::get,
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::crypto::{E2eeChannel, EnableE2eeRequest, EncryptedMessage, SendEncryptedMessageRequest},
};
use nexus_db::repository::keystore;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};
use nexus_common::gateway_event::GatewayEvent;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/channels/{channel_id}/encrypted-messages",
            get(list_encrypted_messages).post(send_encrypted_message),
        )
        .route(
            "/channels/{channel_id}/e2ee",
            get(get_e2ee_config).put(enable_e2ee),
        )
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

#[derive(Deserialize)]
struct MessagesQuery {
    before_sequence: Option<i64>,
    limit: Option<i64>,
}

// ============================================================
// GET /channels/:channel_id/encrypted-messages
// ============================================================

async fn list_encrypted_messages(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Query(params): Query<MessagesQuery>,
) -> NexusResult<Json<Vec<EncryptedMessage>>> {
    let limit = params.limit.unwrap_or(50).min(100);
    let msgs = keystore::list_encrypted_messages(
        &state.db.pg,
        channel_id,
        params.before_sequence,
        limit,
    )
    .await
    .map_err(|e| NexusError::Internal(e))?;
    Ok(Json(msgs))
}

// ============================================================
// POST /channels/:channel_id/encrypted-messages
// ============================================================

async fn send_encrypted_message(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<SendEncryptedMessageRequest>,
) -> NexusResult<Json<EncryptedMessage>> {
    // Verify ciphertext_map is a JSON object
    if !body.ciphertext_map.is_object() {
        return Err(NexusError::Validation {
            message: "ciphertext_map must be a JSON object".into(),
        });
    }

    // Require at least one recipient
    if body.ciphertext_map.as_object().map(|m| m.is_empty()).unwrap_or(true) {
        return Err(NexusError::Validation {
            message: "ciphertext_map must contain at least one recipient".into(),
        });
    }

    // The sender's device_id must be among the keys (they encrypt for themselves too)
    // We derive sender_device_id from a request header or from the first key that
    // matches auth.user_id's devices. For simplicity, clients pass it as a query param
    // or we require it embedded. Here we accept it from the ciphertext_map structure —
    // the sender MUST include their own device in the map.
    // We pick the first device registered to this user as the "sender device".
    let devices = nexus_db::repository::keystore::list_devices(&state.db.pg, auth.user_id)
        .await
        .map_err(|e| NexusError::Internal(e))?;

    let sender_device = devices.into_iter().next().ok_or(NexusError::Validation {
        message: "No device registered for sender. Register a device before sending E2EE messages.".into(),
    })?;

    let msg = keystore::store_encrypted_message(
        &state.db.pg,
        channel_id,
        auth.user_id,
        sender_device.id,
        &body.ciphertext_map,
        body.attachment_meta.as_ref(),
        body.client_ts,
    )
    .await
    .map_err(|e| NexusError::Internal(e))?;

    // Broadcast to gateway (clients receive the ciphertext_map and decrypt locally)
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "ENCRYPTED_MESSAGE_CREATE".into(),
        data: serde_json::json!({
            "id": msg.id,
            "channel_id": msg.channel_id,
            "sender_id": msg.sender_id,
            "sender_device_id": msg.sender_device_id,
            "ciphertext_map": msg.ciphertext_map,
            "attachment_meta": msg.attachment_meta,
            "sequence": msg.sequence,
            "client_ts": msg.client_ts,
            "created_at": msg.created_at,
        }),
        server_id: None,
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(msg))
}

// ============================================================
// GET /channels/:channel_id/e2ee
// ============================================================

async fn get_e2ee_config(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Option<E2eeChannel>>> {
    let config = keystore::get_e2ee_channel(&state.db.pg, channel_id)
        .await
        .map_err(|e| NexusError::Internal(e))?;
    Ok(Json(config))
}

// ============================================================
// PUT /channels/:channel_id/e2ee — Enable E2EE
// ============================================================

async fn enable_e2ee(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<EnableE2eeRequest>,
) -> NexusResult<Json<E2eeChannel>> {
    let rotation_secs = body.rotation_interval_secs.unwrap_or(604_800); // 7 days

    if rotation_secs < 3600 {
        return Err(NexusError::Validation {
            message: "rotation_interval_secs must be at least 3600 (1 hour)".into(),
        });
    }

    let config = keystore::enable_e2ee_channel(&state.db.pg, channel_id, auth.user_id, rotation_secs)
        .await
        .map_err(|e| NexusError::Internal(e))?;

    // Notify channel members that E2EE is now active
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "CHANNEL_E2EE_ENABLED".into(),
        data: serde_json::json!({
            "channel_id": channel_id,
            "enabled_by": auth.user_id,
            "rotation_interval_secs": config.rotation_interval_secs,
        }),
        server_id: None,
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(config))
}
