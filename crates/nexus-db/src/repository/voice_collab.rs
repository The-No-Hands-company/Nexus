//! Repository: voice & collaboration — Phase 22.

use nexus_common::models::voice_collab::*;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::*;

// ── 22-01: Video Layouts ──────────────────────────────────────────────────

pub async fn upsert_video_layout(
    pool: &AnyPool,
    id: Uuid,
    channel_id: Uuid,
    user_id: Uuid,
    layout_type: &str,
    pinned_users: &serde_json::Value,
    custom_positions: &serde_json::Value,
    pip_enabled: bool,
) -> Result<VideoLayout, sqlx::Error> {
    let q = format!(
        "INSERT INTO video_layouts \
         (id, channel_id, user_id, layout_type, pinned_users, custom_positions, pip_enabled) \
         VALUES ($1, $2, $3, $4, $5::jsonb, $6::jsonb, $7) \
         ON CONFLICT (channel_id, user_id) DO UPDATE SET \
           layout_type = $4, pinned_users = $5::jsonb, custom_positions = $6::jsonb, \
           pip_enabled = $7, updated_at = now() \
         RETURNING {VIDEO_LAYOUT_COLS}"
    );
    sqlx::query_as::<_, VideoLayout>(&q)
        .bind(id.to_string())
        .bind(channel_id.to_string())
        .bind(user_id.to_string())
        .bind(layout_type)
        .bind(pinned_users.to_string())
        .bind(custom_positions.to_string())
        .bind(pip_enabled)
        .fetch_one(pool)
        .await
}

pub async fn get_video_layout(
    pool: &AnyPool,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<Option<VideoLayout>, sqlx::Error> {
    let q = format!(
        "SELECT {VIDEO_LAYOUT_COLS} FROM video_layouts WHERE channel_id = $1 AND user_id = $2"
    );
    sqlx::query_as::<_, VideoLayout>(&q)
        .bind(channel_id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(pool)
        .await
}

// ── Virtual Backgrounds ───────────────────────────────────────────────────

pub async fn create_virtual_background(
    pool: &AnyPool,
    id: Uuid,
    user_id: Uuid,
    name: &str,
    bg_type: &str,
    image_url: Option<&str>,
) -> Result<VirtualBackground, sqlx::Error> {
    let q = format!(
        "INSERT INTO virtual_backgrounds (id, user_id, name, bg_type, image_url) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING {VIRTUAL_BACKGROUND_COLS}"
    );
    sqlx::query_as::<_, VirtualBackground>(&q)
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(name)
        .bind(bg_type)
        .bind(image_url)
        .fetch_one(pool)
        .await
}

pub async fn list_virtual_backgrounds(
    pool: &AnyPool,
    user_id: Uuid,
) -> Result<Vec<VirtualBackground>, sqlx::Error> {
    let q = format!(
        "SELECT {VIRTUAL_BACKGROUND_COLS} FROM virtual_backgrounds WHERE user_id = $1 ORDER BY created_at"
    );
    sqlx::query_as::<_, VirtualBackground>(&q)
        .bind(user_id.to_string())
        .fetch_all(pool)
        .await
}

// ── 22-02: Live Streams ───────────────────────────────────────────────────

pub async fn create_live_stream(
    pool: &AnyPool,
    id: Uuid,
    channel_id: Uuid,
    streamer_id: Uuid,
    title: &str,
    is_e2ee: bool,
) -> Result<LiveStream, sqlx::Error> {
    let q = format!(
        "INSERT INTO live_streams (id, channel_id, streamer_id, title, is_e2ee) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING {LIVE_STREAM_COLS}"
    );
    sqlx::query_as::<_, LiveStream>(&q)
        .bind(id.to_string())
        .bind(channel_id.to_string())
        .bind(streamer_id.to_string())
        .bind(title)
        .bind(is_e2ee)
        .fetch_one(pool)
        .await
}

pub async fn list_live_streams(
    pool: &AnyPool,
    channel_id: Uuid,
) -> Result<Vec<LiveStream>, sqlx::Error> {
    let q = format!(
        "SELECT {LIVE_STREAM_COLS} FROM live_streams WHERE channel_id = $1 AND status != 'ended' ORDER BY started_at DESC"
    );
    sqlx::query_as::<_, LiveStream>(&q)
        .bind(channel_id.to_string())
        .fetch_all(pool)
        .await
}

pub async fn end_live_stream(
    pool: &AnyPool,
    id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE live_streams SET status = 'ended', ended_at = now() WHERE id = $1")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

// ── 22-03: Breakout Rooms ─────────────────────────────────────────────────

pub async fn create_breakout_room(
    pool: &AnyPool,
    id: Uuid,
    parent_channel: Uuid,
    name: &str,
    capacity: i32,
    created_by: Uuid,
) -> Result<BreakoutRoom, sqlx::Error> {
    let q = format!(
        "INSERT INTO breakout_rooms (id, parent_channel, name, capacity, created_by) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING {BREAKOUT_ROOM_COLS}"
    );
    sqlx::query_as::<_, BreakoutRoom>(&q)
        .bind(id.to_string())
        .bind(parent_channel.to_string())
        .bind(name)
        .bind(capacity)
        .bind(created_by.to_string())
        .fetch_one(pool)
        .await
}

pub async fn list_breakout_rooms(
    pool: &AnyPool,
    parent_channel: Uuid,
) -> Result<Vec<BreakoutRoom>, sqlx::Error> {
    let q = format!(
        "SELECT {BREAKOUT_ROOM_COLS} FROM breakout_rooms WHERE parent_channel = $1 AND status = 'open'"
    );
    sqlx::query_as::<_, BreakoutRoom>(&q)
        .bind(parent_channel.to_string())
        .fetch_all(pool)
        .await
}

// ── Collab Sessions ───────────────────────────────────────────────────────

pub async fn create_collab_session(
    pool: &AnyPool,
    id: Uuid,
    channel_id: Uuid,
    session_type: &str,
    document_id: Option<Uuid>,
) -> Result<CollabSession, sqlx::Error> {
    let q = format!(
        "INSERT INTO collab_sessions (id, channel_id, session_type, document_id) \
         VALUES ($1, $2, $3, $4) \
         RETURNING {COLLAB_SESSION_COLS}"
    );
    sqlx::query_as::<_, CollabSession>(&q)
        .bind(id.to_string())
        .bind(channel_id.to_string())
        .bind(session_type)
        .bind(document_id.map(|u| u.to_string()))
        .fetch_one(pool)
        .await
}

pub async fn list_collab_sessions(
    pool: &AnyPool,
    channel_id: Uuid,
) -> Result<Vec<CollabSession>, sqlx::Error> {
    let q = format!(
        "SELECT {COLLAB_SESSION_COLS} FROM collab_sessions WHERE channel_id = $1 AND is_active = TRUE"
    );
    sqlx::query_as::<_, CollabSession>(&q)
        .bind(channel_id.to_string())
        .fetch_all(pool)
        .await
}

// ── 22-04: Spatial Audio ──────────────────────────────────────────────────

pub async fn upsert_spatial_audio_config(
    pool: &AnyPool,
    id: Uuid,
    channel_id: Uuid,
    preset: &str,
    room_width: f32,
    room_depth: f32,
    room_height: f32,
    positions: &serde_json::Value,
    hrtf_enabled: bool,
    ambisonics_order: i32,
) -> Result<SpatialAudioConfig, sqlx::Error> {
    let q = format!(
        "INSERT INTO spatial_audio_configs \
         (id, channel_id, preset, room_width, room_depth, room_height, positions, hrtf_enabled, ambisonics_order) \
         VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8, $9) \
         ON CONFLICT (channel_id) DO UPDATE SET \
           preset = $3, room_width = $4, room_depth = $5, room_height = $6, \
           positions = $7::jsonb, hrtf_enabled = $8, ambisonics_order = $9, updated_at = now() \
         RETURNING {SPATIAL_AUDIO_CONFIG_COLS}"
    );
    sqlx::query_as::<_, SpatialAudioConfig>(&q)
        .bind(id.to_string())
        .bind(channel_id.to_string())
        .bind(preset)
        .bind(room_width as f64)
        .bind(room_depth as f64)
        .bind(room_height as f64)
        .bind(positions.to_string())
        .bind(hrtf_enabled)
        .bind(ambisonics_order)
        .fetch_one(pool)
        .await
}

pub async fn get_spatial_audio_config(
    pool: &AnyPool,
    channel_id: Uuid,
) -> Result<Option<SpatialAudioConfig>, sqlx::Error> {
    let q = format!(
        "SELECT {SPATIAL_AUDIO_CONFIG_COLS} FROM spatial_audio_configs WHERE channel_id = $1"
    );
    sqlx::query_as::<_, SpatialAudioConfig>(&q)
        .bind(channel_id.to_string())
        .fetch_optional(pool)
        .await
}

// ── 22-05: Voice Presets ──────────────────────────────────────────────────

pub async fn list_voice_presets(
    pool: &AnyPool,
) -> Result<Vec<VoicePreset>, sqlx::Error> {
    let q = format!(
        "SELECT {VOICE_PRESET_COLS} FROM voice_presets ORDER BY name"
    );
    sqlx::query_as::<_, VoicePreset>(&q)
        .fetch_all(pool)
        .await
}

pub async fn get_voice_preset(
    pool: &AnyPool,
    name: &str,
) -> Result<Option<VoicePreset>, sqlx::Error> {
    let q = format!(
        "SELECT {VOICE_PRESET_COLS} FROM voice_presets WHERE name = $1"
    );
    sqlx::query_as::<_, VoicePreset>(&q)
        .bind(name)
        .fetch_optional(pool)
        .await
}
