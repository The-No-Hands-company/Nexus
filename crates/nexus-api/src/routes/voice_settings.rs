//! Voice settings & music queue routes — 17-04: Advanced Voice Features.
//!
//! GET    /users/@me/voice-settings             — Get voice settings
//! PATCH  /users/@me/voice-settings             — Update voice settings
//!
//! POST   /channels/:id/music-queue             — Add to music queue
//! GET    /channels/:id/music-queue             — List queue
//! PATCH  /music-queue/:id                      — Update queue item status
//! POST   /channels/:id/music-queue/skip        — Skip current track
//! DELETE /channels/:id/music-queue              — Clear queue

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    middleware,
    routing::{get, patch, post},
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::snowflake;
use nexus_db::repository::voice_settings;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{AppState, middleware::AuthContext};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Voice settings (per-user preferences)
        .route(
            "/users/@me/voice-settings",
            get(get_settings).patch(update_settings),
        )
        // Music queue
        .route(
            "/channels/{channel_id}/music-queue",
            post(add_to_queue).get(list_queue).delete(clear_queue),
        )
        .route("/music-queue/{id}", patch(update_queue_status))
        .route("/channels/{channel_id}/music-queue/skip", post(skip_track))
        .route_layer(middleware::from_fn(
            crate::middleware::combined_auth_middleware,
        ))
}

// ── Request / Response ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct UpdateSettingsRequest {
    spatial_audio: Option<bool>,
    noise_gate_enabled: Option<bool>,
    noise_gate_threshold: Option<f32>,
    echo_cancel_level: Option<String>,
    auto_gain: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AddToQueueRequest {
    title: String,
    source_url: String,
    duration_ms: i32,
}

#[derive(Debug, Deserialize)]
struct UpdateQueueStatusRequest {
    status: String,
}

// ── Voice Settings Handlers ───────────────────────────────────────────────────

async fn get_settings(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<nexus_common::models::multimedia::VoiceSettings>> {
    let settings = voice_settings::get_voice_settings(&state.db.pool, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    match settings {
        Some(s) => Ok(Json(s)),
        None => {
            // Create defaults
            let s = voice_settings::upsert_voice_settings(
                &state.db.pool,
                ctx.user_id,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| NexusError::Internal(e.into()))?;
            Ok(Json(s))
        }
    }
}

async fn update_settings(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<UpdateSettingsRequest>,
) -> NexusResult<Json<nexus_common::models::multimedia::VoiceSettings>> {
    if let Some(ref level) = body.echo_cancel_level
        && !matches!(level.as_str(), "off" | "low" | "moderate" | "aggressive") {
            return Err(NexusError::Validation {
                message: "echo_cancel_level must be off, low, moderate, or aggressive".into(),
            });
        }
    if let Some(threshold) = body.noise_gate_threshold
        && !(-100.0..=0.0).contains(&threshold) {
            return Err(NexusError::Validation {
                message: "noise_gate_threshold must be between -100.0 and 0.0 dB".into(),
            });
        }

    let settings = voice_settings::upsert_voice_settings(
        &state.db.pool,
        ctx.user_id,
        body.spatial_audio,
        body.noise_gate_enabled,
        body.noise_gate_threshold,
        body.echo_cancel_level.as_deref(),
        body.auto_gain,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(settings))
}

// ── Music Queue Handlers ──────────────────────────────────────────────────────

async fn add_to_queue(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<AddToQueueRequest>,
) -> NexusResult<Json<nexus_common::models::multimedia::VoiceMusicQueueItem>> {
    if body.title.is_empty() || body.title.len() > 500 {
        return Err(NexusError::Validation {
            message: "Title must be 1-500 characters".into(),
        });
    }
    if body.duration_ms <= 0 {
        return Err(NexusError::Validation {
            message: "Duration must be positive".into(),
        });
    }

    let item = voice_settings::add_to_queue(
        &state.db.pool,
        snowflake::generate_id(),
        channel_id,
        ctx.user_id,
        &body.title,
        &body.source_url,
        body.duration_ms,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(item))
}

async fn list_queue(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Vec<nexus_common::models::multimedia::VoiceMusicQueueItem>>> {
    voice_settings::list_queue(&state.db.pool, channel_id)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn update_queue_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateQueueStatusRequest>,
) -> NexusResult<Json<nexus_common::models::multimedia::VoiceMusicQueueItem>> {
    if !matches!(
        body.status.as_str(),
        "queued" | "playing" | "played" | "skipped"
    ) {
        return Err(NexusError::Validation {
            message: "status must be queued, playing, played, or skipped".into(),
        });
    }
    voice_settings::update_queue_status(&state.db.pool, id, &body.status)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn skip_track(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let next = voice_settings::skip_current(&state.db.pool, channel_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "now_playing": next
    })))
}

async fn clear_queue(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let count = voice_settings::clear_queue(&state.db.pool, channel_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(serde_json::json!({ "ok": true, "cleared": count })))
}
