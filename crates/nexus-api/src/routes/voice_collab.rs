//! Voice & real-time collaboration routes — Phase 22.
//!
//! Video layouts, virtual backgrounds, live streams, breakout rooms,
//! collab sessions, spatial audio, voice presets.

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::get,
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::models::voice_collab::*;
use nexus_db::repository::{voice_collab, members};
use nexus_common::models::Member;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// ── Router ─────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Video layouts
        .route(
            "/channels/:channel_id/video-layout",
            get(get_video_layout).post(upsert_video_layout),
        )
        // Virtual backgrounds
        .route(
            "/users/@me/virtual-backgrounds",
            get(list_virtual_backgrounds).post(create_virtual_background),
        )
        // Live streams
        .route(
            "/servers/:server_id/live-streams",
            get(list_live_streams).post(create_live_stream),
        )
        // Breakout rooms
        .route(
            "/channels/:channel_id/breakout-rooms",
            get(list_breakout_rooms).post(create_breakout_room),
        )
        // Collab sessions
        .route(
            "/channels/:channel_id/collab-sessions",
            get(list_collab_sessions).post(create_collab_session),
        )
        // Spatial audio configs
        .route(
            "/channels/:channel_id/spatial-audio",
            get(get_spatial_audio).post(upsert_spatial_audio),
        )
        // Voice presets (global, read-only)
        .route("/voice-presets", get(list_voice_presets))
        .route("/voice-presets/:preset_id", get(get_voice_preset))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request / Query ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LimitQuery { limit: Option<i64> }

#[derive(Debug, Deserialize)]
struct UpsertVideoLayoutReq {
    layout_type: String,
    pinned_users: Option<serde_json::Value>,
    custom_positions: Option<serde_json::Value>,
    pip_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CreateVirtualBgReq {
    name: String,
    bg_type: String,
    url: Option<String>,
    config: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct CreateLiveStreamReq {
    channel_id: Uuid,
    title: Option<String>,
    is_e2ee: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CreateBreakoutRoomReq {
    name: String,
    capacity: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct CreateCollabSessionReq {
    session_type: String,
    document_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct UpsertSpatialAudioReq {
    preset: Option<String>,
    room_width: Option<f32>,
    room_depth: Option<f32>,
    room_height: Option<f32>,
    positions: Option<serde_json::Value>,
    hrtf_enabled: Option<bool>,
    ambisonics_order: Option<i32>,
}

// ── Handlers ──────────────────────────────────────────────────────────────

async fn get_video_layout(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Option<VideoLayout>>> {
    let row = voice_collab::get_video_layout(&state.db.pool, channel_id, ctx.user_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn upsert_video_layout(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<UpsertVideoLayoutReq>,
) -> NexusResult<Json<VideoLayout>> {
    let row = voice_collab::upsert_video_layout(
        &state.db.pool, Uuid::new_v4(), channel_id, ctx.user_id, &body.layout_type,
        &body.pinned_users.unwrap_or(serde_json::json!([])),
        &body.custom_positions.unwrap_or(serde_json::json!({})),
        body.pip_enabled.unwrap_or(false),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_virtual_backgrounds(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<VirtualBackground>>> {
    let rows = voice_collab::list_virtual_backgrounds(&state.db.pool, ctx.user_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_virtual_background(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<CreateVirtualBgReq>,
) -> NexusResult<Json<VirtualBackground>> {
    let row = voice_collab::create_virtual_background(
        &state.db.pool, Uuid::new_v4(), ctx.user_id,
        &body.name, &body.bg_type, body.url.as_deref(),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

#[derive(Debug, Deserialize)]
struct LiveStreamQuery { channel_id: Uuid }

async fn list_live_streams(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<LiveStreamQuery>,
) -> NexusResult<Json<Vec<LiveStream>>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let rows = voice_collab::list_live_streams(&state.db.pool, q.channel_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_live_stream(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateLiveStreamReq>,
) -> NexusResult<Json<LiveStream>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let row = voice_collab::create_live_stream(
        &state.db.pool, Uuid::new_v4(), body.channel_id,
        ctx.user_id, &body.title.unwrap_or_default(),
        body.is_e2ee.unwrap_or(false),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_breakout_rooms(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Vec<BreakoutRoom>>> {
    let rows = voice_collab::list_breakout_rooms(&state.db.pool, channel_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_breakout_room(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateBreakoutRoomReq>,
) -> NexusResult<Json<BreakoutRoom>> {
    let row = voice_collab::create_breakout_room(
        &state.db.pool, Uuid::new_v4(), channel_id, &body.name,
        body.capacity.unwrap_or(10), ctx.user_id,
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_collab_sessions(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Vec<CollabSession>>> {
    let rows = voice_collab::list_collab_sessions(&state.db.pool, channel_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_collab_session(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateCollabSessionReq>,
) -> NexusResult<Json<CollabSession>> {
    let row = voice_collab::create_collab_session(
        &state.db.pool, Uuid::new_v4(), channel_id, &body.session_type,
        body.document_id,
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn get_spatial_audio(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Option<SpatialAudioConfig>>> {
    let row = voice_collab::get_spatial_audio_config(&state.db.pool, channel_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn upsert_spatial_audio(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<UpsertSpatialAudioReq>,
) -> NexusResult<Json<SpatialAudioConfig>> {
    let row = voice_collab::upsert_spatial_audio_config(
        &state.db.pool, Uuid::new_v4(), channel_id,
        &body.preset.unwrap_or_else(|| "default".into()),
        body.room_width.unwrap_or(10.0),
        body.room_depth.unwrap_or(10.0),
        body.room_height.unwrap_or(3.0),
        &body.positions.unwrap_or(serde_json::json!({})),
        body.hrtf_enabled.unwrap_or(true),
        body.ambisonics_order.unwrap_or(1),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_voice_presets(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<VoicePreset>>> {
    let rows = voice_collab::list_voice_presets(&state.db.pool)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn get_voice_preset(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(preset_name): Path<String>,
) -> NexusResult<Json<VoicePreset>> {
    let row = voice_collab::get_voice_preset(&state.db.pool, &preset_name)
        .await.map_err(|e| NexusError::Internal(e.into()))?
        .ok_or(NexusError::NotFound { resource: "voice_preset".into() })?;
    Ok(Json(row))
}
