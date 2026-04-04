//! Voice API routes — REST endpoints for voice channel operations.
//!
//! These complement the voice WebSocket signaling:
//! - WebSocket handles real-time SDP/ICE exchange and media
//! - REST handles state queries, moderation actions, and channel management
//!
//! Routes:
//! - GET  /voice/channels/{channel_id}        — Get voice channel state (who's connected)
//! - POST /voice/channels/{channel_id}/join    — Join a voice channel (pre-flight)
//! - POST /voice/channels/{channel_id}/leave   — Leave a voice channel
//! - PATCH /voice/state                        — Update own voice state
//! - POST /voice/channels/{channel_id}/mute    — Server mute/deaf a user (mod action)
//! - GET  /voice/stats                         — Voice server statistics

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{get, patch, post},
    Json, Router,
};
use chrono::Utc;
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::GatewayEvent,
};
use nexus_voice::state::{VoiceGlobalStats, VoiceModAction, VoiceState, VoiceStateUpdate};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

/// Voice routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Voice channel state
        .route(
            "/voice/channels/{channel_id}",
            get(get_voice_channel_state),
        )
        // Join pre-flight (validates permissions, returns voice server info)
        .route(
            "/voice/channels/{channel_id}/join",
            post(voice_join_preflight),
        )
        // Leave voice channel via REST
        .route(
            "/voice/channels/{channel_id}/leave",
            post(voice_leave),
        )
        // Update own voice state
        .route("/voice/state", patch(update_voice_state))
        // Server mute/deaf (moderation)
        .route(
            "/voice/channels/{channel_id}/mute",
            post(server_mute),
        )
        // Voice stats
        .route("/voice/stats", get(voice_stats))
        // All voice routes require authentication
        .layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

/// Response for voice channel state.
#[derive(Debug, Serialize)]
pub struct VoiceChannelResponse {
    pub channel_id: Uuid,
    pub voice_states: Vec<VoiceState>,
    pub participant_count: usize,
}

/// Response for join pre-flight.
#[derive(Debug, Serialize)]
pub struct VoiceJoinResponse {
    /// Voice WebSocket URL to connect to for signaling.
    pub voice_ws_url: String,
    /// Session info for the voice connection.
    pub session_id: String,
    /// Current participants in the channel.
    pub participants: Vec<VoiceState>,
}

/// GET /voice/channels/{channel_id} — Get voice state for a channel.
async fn get_voice_channel_state(
    State(state): State<Arc<AppState>>,
    Extension(_auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<VoiceChannelResponse>> {
    metrics::counter!("nexus_voice_requests_total", "route" => "channel_state").increment(1);

    // Verify channel exists
    let _channel = nexus_db::repository::channels::find_by_id(&state.db.pool, channel_id)
        .await
        .map_err(NexusError::Database)?
        .ok_or_else(|| NexusError::NotFound { resource: "Channel".into() })?;

    // Get voice states for this channel
    let voice_states = state.voice_state.get_channel_members(channel_id).await;

    Ok(Json(VoiceChannelResponse {
        channel_id,
        participant_count: voice_states.len(),
        voice_states,
    }))
}

/// POST /voice/channels/{channel_id}/join — Pre-flight for joining voice.
///
/// Validates permissions and returns the voice WebSocket URL.
/// The actual connection happens via the voice WebSocket.
async fn voice_join_preflight(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<VoiceJoinResponse>> {
    metrics::counter!("nexus_voice_requests_total", "route" => "join_preflight").increment(1);

    // Verify channel exists and is a voice channel
    let channel = nexus_db::repository::channels::find_by_id(&state.db.pool, channel_id)
        .await
        .map_err(NexusError::Database)?
        .ok_or_else(|| NexusError::NotFound { resource: "Channel".into() })?;

    // Check it's a voice-capable channel
    match channel.channel_type {
        nexus_common::models::channel::ChannelType::Voice
        | nexus_common::models::channel::ChannelType::Stage => {}
        _ => {
            metrics::counter!("nexus_voice_requests_total", "route" => "join_preflight", "outcome" => "invalid_channel_type").increment(1);
            return Err(NexusError::Validation {
                message: "Channel is not a voice channel".into(),
            });
        }
    }

    // Check user limit (0 = unlimited)
    if let Some(limit) = channel.user_limit {
        if limit > 0 {
            let current_count = state.voice_state.get_channel_count(channel_id).await;
            if current_count >= limit as usize {
                metrics::counter!("nexus_voice_requests_total", "route" => "join_preflight", "outcome" => "full").increment(1);
                return Err(NexusError::LimitReached {
                    message: "Voice channel is full".into(),
                });
            }
        }
    }

    // If it's a server channel, check membership and timeout status
    if let Some(server_id) = channel.server_id {
        let member =
            nexus_db::repository::members::find_member(&state.db.pool, auth.user_id, server_id)
                .await
                .map_err(NexusError::Database)?
                .ok_or(NexusError::Forbidden)?;

        // Timeout enforcement: timed-out members cannot join voice
        if let Some(disabled_until) = member.communication_disabled_until {
            if disabled_until > Utc::now() {
                metrics::counter!("nexus_voice_requests_total", "route" => "join_preflight", "outcome" => "timeout_blocked").increment(1);
                return Err(NexusError::Validation {
                    message: "You are currently timed out in this server".into(),
                });
            }
        }
    }

    let config = nexus_common::config::get();
    let voice_ws_url = format!(
        "ws://{}:{}/voice",
        config.server.host, config.server.voice_port
    );
    let session_id = Uuid::new_v4().to_string();

    // Get current participants
    let participants = state.voice_state.get_channel_members(channel_id).await;

    Ok(Json(VoiceJoinResponse {
        voice_ws_url,
        session_id,
        participants,
    }))
}

/// POST /voice/channels/{channel_id}/leave — Leave voice via REST.
///
/// This is an alternative to disconnecting from the voice WebSocket.
/// Useful when the client wants to leave voice but keep the signaling connection.
async fn voice_leave(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    metrics::counter!("nexus_voice_requests_total", "route" => "leave").increment(1);

    // Check if user is actually in this channel
    let voice_state = state
        .voice_state
        .get_user_state(auth.user_id)
        .await
        .ok_or_else(|| NexusError::Validation { message: "Not in a voice channel".into() })?;

    if voice_state.channel_id != channel_id {
        metrics::counter!("nexus_voice_requests_total", "route" => "leave", "outcome" => "wrong_channel").increment(1);
        return Err(NexusError::Validation {
            message: "Not in the specified voice channel".into(),
        });
    }

    // Remove from voice state
    state.voice_state.leave(auth.user_id).await;

    // Broadcast leave event
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "VOICE_STATE_UPDATE".into(),
        data: serde_json::json!({
            "user_id": auth.user_id,
            "channel_id": null,
            "server_id": voice_state.server_id,
            "session_id": voice_state.session_id,
        }),
        server_id: voice_state.server_id,
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "success": true })))
}

/// PATCH /voice/state — Update own voice state (mute/deaf/video/stream).
async fn update_voice_state(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(update): Json<VoiceStateUpdate>,
) -> NexusResult<Json<VoiceState>> {
    metrics::counter!("nexus_voice_requests_total", "route" => "update_state").increment(1);

    let new_state = state
        .voice_state
        .update_self_state(auth.user_id, &update)
        .await
        .ok_or_else(|| NexusError::Validation { message: "Not in a voice channel".into() })?;

    // Broadcast state change
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "VOICE_STATE_UPDATE".into(),
        data: serde_json::to_value(&new_state).unwrap_or_default(),
        server_id: new_state.server_id,
        channel_id: Some(new_state.channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(new_state))
}

/// POST /voice/channels/{channel_id}/mute — Server mute/deaf a user (mod action).
async fn server_mute(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(action): Json<VoiceModAction>,
) -> NexusResult<Json<VoiceState>> {
    metrics::counter!("nexus_voice_requests_total", "route" => "server_mute").increment(1);

    // Verify the channel exists and is in a server
    let channel = nexus_db::repository::channels::find_by_id(&state.db.pool, channel_id)
        .await
        .map_err(NexusError::Database)?
        .ok_or_else(|| NexusError::NotFound { resource: "Channel".into() })?;

    let server_id = channel
        .server_id
        .ok_or_else(|| NexusError::Validation { message: "Not a server channel".into() })?;

    // Check the moderator has the MUTE_MEMBERS permission.
    let server = nexus_db::repository::servers::find_by_id(&state.db.pool, server_id)
        .await
        .map_err(NexusError::Database)?
        .ok_or_else(|| NexusError::NotFound { resource: "Server".into() })?;

    // Server owner always passes; otherwise evaluate role permissions.
    if server.owner_id != auth.user_id {
        let member =
            nexus_db::repository::members::find_member(&state.db.pool, auth.user_id, server_id)
                .await
                .map_err(NexusError::Database)?
                .ok_or(NexusError::Forbidden)?;

        let all_roles =
            nexus_db::repository::roles::list_server_roles(&state.db.pool, server_id)
                .await
                .map_err(NexusError::Database)?;

        let effective = all_roles.iter().fold(
            nexus_common::models::permissions::Permissions::empty(),
            |acc, role| {
                if role.is_default || member.roles.contains(&role.id) {
                    acc | nexus_common::models::permissions::Permissions::from_bits_truncate(
                        role.permissions,
                    )
                } else {
                    acc
                }
            },
        );

        if !effective.has(nexus_common::models::permissions::Permissions::MUTE_MEMBERS) {
            metrics::counter!(
                "nexus_voice_requests_total",
                "route" => "server_mute",
                "outcome" => "forbidden"
            )
            .increment(1);
            return Err(NexusError::MissingPermission {
                permission: "MUTE_MEMBERS".into(),
            });
        }
    }

    // Check target is in this channel
    let target_state = state
        .voice_state
        .get_user_state(action.target_user_id)
        .await
        .ok_or_else(|| NexusError::Validation { message: "Target user not in voice".into() })?;

    if target_state.channel_id != channel_id {
        metrics::counter!("nexus_voice_requests_total", "route" => "server_mute", "outcome" => "target_wrong_channel").increment(1);
        return Err(NexusError::Validation {
            message: "Target user not in this channel".into(),
        });
    }

    // Apply the mod action
    let new_state = state
        .voice_state
        .apply_mod_action(&action)
        .await
        .ok_or_else(|| NexusError::Internal(anyhow::anyhow!("Voice state not found after apply")))?;

    // Broadcast state change
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "VOICE_STATE_UPDATE".into(),
        data: serde_json::to_value(&new_state).unwrap_or_default(),
        server_id: Some(server_id),
        channel_id: Some(channel_id),
        user_id: Some(action.target_user_id),
    });

    Ok(Json(new_state))
}

/// GET /voice/stats — Voice server statistics (admin).
async fn voice_stats(
    State(state): State<Arc<AppState>>,
    Extension(_auth): Extension<AuthContext>,
) -> NexusResult<Json<VoiceGlobalStats>> {
    metrics::counter!("nexus_voice_requests_total", "route" => "stats").increment(1);
    let stats = state.voice_state.stats().await;
    Ok(Json(stats))
}
