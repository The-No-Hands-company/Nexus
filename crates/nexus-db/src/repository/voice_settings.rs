//! Voice settings & music queue repository queries.

use nexus_common::models::multimedia::{VoiceMusicQueueItem, VoiceSettings};
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::VOICE_MUSIC_QUEUE_COLS;

// ── Voice Settings ────────────────────────────────────────────────────────────

const VS_COLS: &str =
    "user_id::text AS user_id, spatial_audio, noise_gate_enabled, \
     noise_gate_threshold, echo_cancel_level, auto_gain, \
     updated_at::text AS updated_at";

pub async fn get_voice_settings(
    pool: &AnyPool,
    user_id: Uuid,
) -> Result<Option<VoiceSettings>, sqlx::Error> {
    let q = format!("SELECT {VS_COLS} FROM voice_settings WHERE user_id = $1");
    sqlx::query_as::<_, VoiceSettings>(&q)
        .bind(user_id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn upsert_voice_settings(
    pool: &AnyPool,
    user_id: Uuid,
    spatial_audio: Option<bool>,
    noise_gate_enabled: Option<bool>,
    noise_gate_threshold: Option<f32>,
    echo_cancel_level: Option<&str>,
    auto_gain: Option<bool>,
) -> Result<VoiceSettings, sqlx::Error> {
    let q = format!(
        "INSERT INTO voice_settings (user_id, spatial_audio, noise_gate_enabled, \
         noise_gate_threshold, echo_cancel_level, auto_gain) \
         VALUES ($1, COALESCE($2, false), COALESCE($3, true), \
         COALESCE($4, -50.0), COALESCE($5, 'moderate'), COALESCE($6, true)) \
         ON CONFLICT (user_id) DO UPDATE SET \
         spatial_audio = COALESCE($2, voice_settings.spatial_audio), \
         noise_gate_enabled = COALESCE($3, voice_settings.noise_gate_enabled), \
         noise_gate_threshold = COALESCE($4, voice_settings.noise_gate_threshold), \
         echo_cancel_level = COALESCE($5, voice_settings.echo_cancel_level), \
         auto_gain = COALESCE($6, voice_settings.auto_gain), \
         updated_at = NOW() \
         RETURNING {VS_COLS}"
    );
    // Cast f32 threshold to f64 for sqlx bind compatibility
    let threshold_f64 = noise_gate_threshold.map(|v| v as f64);
    sqlx::query_as::<_, VoiceSettings>(&q)
        .bind(user_id.to_string())
        .bind(spatial_audio)
        .bind(noise_gate_enabled)
        .bind(threshold_f64)
        .bind(echo_cancel_level)
        .bind(auto_gain)
        .fetch_one(pool)
        .await
}

// ── Music Queue ───────────────────────────────────────────────────────────────

pub async fn add_to_queue(
    pool: &AnyPool,
    id: Uuid,
    channel_id: Uuid,
    added_by: Uuid,
    title: &str,
    source_url: &str,
    duration_ms: i32,
) -> Result<VoiceMusicQueueItem, sqlx::Error> {
    let q = format!(
        "INSERT INTO voice_music_queue (id, channel_id, added_by, title, source_url, \
         duration_ms, position, status) \
         VALUES ($1, $2, $3, $4, $5, $6, \
         (SELECT COALESCE(MAX(position), 0) + 1 FROM voice_music_queue WHERE channel_id = $2), \
         'queued') \
         RETURNING {VOICE_MUSIC_QUEUE_COLS}"
    );
    sqlx::query_as::<_, VoiceMusicQueueItem>(&q)
        .bind(id.to_string())
        .bind(channel_id.to_string())
        .bind(added_by.to_string())
        .bind(title)
        .bind(source_url)
        .bind(duration_ms)
        .fetch_one(pool)
        .await
}

pub async fn list_queue(
    pool: &AnyPool,
    channel_id: Uuid,
) -> Result<Vec<VoiceMusicQueueItem>, sqlx::Error> {
    let q = format!(
        "SELECT {VOICE_MUSIC_QUEUE_COLS} FROM voice_music_queue \
         WHERE channel_id = $1 AND status IN ('queued', 'playing') \
         ORDER BY position"
    );
    sqlx::query_as::<_, VoiceMusicQueueItem>(&q)
        .bind(channel_id.to_string())
        .fetch_all(pool)
        .await
}

pub async fn update_queue_status(
    pool: &AnyPool,
    id: Uuid,
    status: &str,
) -> Result<VoiceMusicQueueItem, sqlx::Error> {
    let q = format!(
        "UPDATE voice_music_queue SET status = $2 \
         WHERE id = $1 RETURNING {VOICE_MUSIC_QUEUE_COLS}"
    );
    sqlx::query_as::<_, VoiceMusicQueueItem>(&q)
        .bind(id.to_string())
        .bind(status)
        .fetch_one(pool)
        .await
}

pub async fn skip_current(
    pool: &AnyPool,
    channel_id: Uuid,
) -> Result<Option<VoiceMusicQueueItem>, sqlx::Error> {
    // Mark the currently playing item as skipped
    sqlx::query(
        "UPDATE voice_music_queue SET status = 'skipped' \
         WHERE channel_id = $1 AND status = 'playing'",
    )
    .bind(channel_id.to_string())
    .execute(pool)
    .await?;
    // Pick the next queued item and set it to playing
    let q = format!(
        "UPDATE voice_music_queue SET status = 'playing' \
         WHERE id = (SELECT id FROM voice_music_queue \
         WHERE channel_id = $1 AND status = 'queued' ORDER BY position LIMIT 1) \
         RETURNING {VOICE_MUSIC_QUEUE_COLS}"
    );
    sqlx::query_as::<_, VoiceMusicQueueItem>(&q)
        .bind(channel_id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn clear_queue(
    pool: &AnyPool,
    channel_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "DELETE FROM voice_music_queue WHERE channel_id = $1 AND status = 'queued'",
    )
    .bind(channel_id.to_string())
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}
