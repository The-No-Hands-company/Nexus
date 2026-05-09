//! Voice & video note repository queries.

use nexus_common::models::multimedia::{VideoNote, VoiceNote};
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::{VIDEO_NOTE_COLS, VOICE_NOTE_COLS};

// ── Voice Notes ───────────────────────────────────────────────────────────────

pub async fn create_voice_note(
    pool: &AnyPool,
    id: Uuid,
    channel_id: Uuid,
    author_id: Uuid,
    storage_key: &str,
    filename: &str,
    content_type: &str,
    size: i64,
    duration_ms: i32,
    waveform: Option<&str>,
    message_id: Option<Uuid>,
) -> Result<VoiceNote, sqlx::Error> {
    let q = format!(
        "INSERT INTO voice_notes (id, channel_id, author_id, storage_key, filename, \
         content_type, size, duration_ms, waveform, message_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         RETURNING {VOICE_NOTE_COLS}"
    );
    sqlx::query_as::<_, VoiceNote>(&q)
        .bind(id.to_string())
        .bind(channel_id.to_string())
        .bind(author_id.to_string())
        .bind(storage_key)
        .bind(filename)
        .bind(content_type)
        .bind(size)
        .bind(duration_ms)
        .bind(waveform)
        .bind(message_id.map(|u| u.to_string()))
        .fetch_one(pool)
        .await
}

pub async fn list_voice_notes(
    pool: &AnyPool,
    channel_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<VoiceNote>, sqlx::Error> {
    let q = format!(
        "SELECT {VOICE_NOTE_COLS} FROM voice_notes \
         WHERE channel_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    );
    sqlx::query_as::<_, VoiceNote>(&q)
        .bind(channel_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

pub async fn find_voice_note(pool: &AnyPool, id: Uuid) -> Result<Option<VoiceNote>, sqlx::Error> {
    let q = format!("SELECT {VOICE_NOTE_COLS} FROM voice_notes WHERE id = $1");
    sqlx::query_as::<_, VoiceNote>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn set_voice_note_transcript(
    pool: &AnyPool,
    id: Uuid,
    transcript: &str,
) -> Result<VoiceNote, sqlx::Error> {
    let q =
        format!("UPDATE voice_notes SET transcript = $2 WHERE id = $1 RETURNING {VOICE_NOTE_COLS}");
    sqlx::query_as::<_, VoiceNote>(&q)
        .bind(id.to_string())
        .bind(transcript)
        .fetch_one(pool)
        .await
}

#[derive(sqlx::FromRow)]
struct StorageKeyRow {
    storage_key: String,
}

pub async fn delete_voice_note(
    pool: &AnyPool,
    id: Uuid,
    author_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query_as::<_, StorageKeyRow>(
        "DELETE FROM voice_notes WHERE id = $1 AND author_id = $2 RETURNING storage_key",
    )
    .bind(id.to_string())
    .bind(author_id.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.storage_key))
}

// ── Video Notes ───────────────────────────────────────────────────────────────

pub async fn create_video_note(
    pool: &AnyPool,
    id: Uuid,
    channel_id: Uuid,
    author_id: Uuid,
    storage_key: &str,
    filename: &str,
    content_type: &str,
    size: i64,
    duration_ms: i32,
    width: Option<i32>,
    height: Option<i32>,
    thumbnail_key: Option<&str>,
    message_id: Option<Uuid>,
) -> Result<VideoNote, sqlx::Error> {
    let q = format!(
        "INSERT INTO video_notes (id, channel_id, author_id, storage_key, filename, \
         content_type, size, duration_ms, width, height, thumbnail_key, message_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
         RETURNING {VIDEO_NOTE_COLS}"
    );
    sqlx::query_as::<_, VideoNote>(&q)
        .bind(id.to_string())
        .bind(channel_id.to_string())
        .bind(author_id.to_string())
        .bind(storage_key)
        .bind(filename)
        .bind(content_type)
        .bind(size)
        .bind(duration_ms)
        .bind(width)
        .bind(height)
        .bind(thumbnail_key)
        .bind(message_id.map(|u| u.to_string()))
        .fetch_one(pool)
        .await
}

pub async fn list_video_notes(
    pool: &AnyPool,
    channel_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<VideoNote>, sqlx::Error> {
    let q = format!(
        "SELECT {VIDEO_NOTE_COLS} FROM video_notes \
         WHERE channel_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    );
    sqlx::query_as::<_, VideoNote>(&q)
        .bind(channel_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

pub async fn find_video_note(pool: &AnyPool, id: Uuid) -> Result<Option<VideoNote>, sqlx::Error> {
    let q = format!("SELECT {VIDEO_NOTE_COLS} FROM video_notes WHERE id = $1");
    sqlx::query_as::<_, VideoNote>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn set_video_note_transcript(
    pool: &AnyPool,
    id: Uuid,
    transcript: &str,
) -> Result<VideoNote, sqlx::Error> {
    let q =
        format!("UPDATE video_notes SET transcript = $2 WHERE id = $1 RETURNING {VIDEO_NOTE_COLS}");
    sqlx::query_as::<_, VideoNote>(&q)
        .bind(id.to_string())
        .bind(transcript)
        .fetch_one(pool)
        .await
}

pub async fn delete_video_note(
    pool: &AnyPool,
    id: Uuid,
    author_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query_as::<_, StorageKeyRow>(
        "DELETE FROM video_notes WHERE id = $1 AND author_id = $2 RETURNING storage_key",
    )
    .bind(id.to_string())
    .bind(author_id.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.storage_key))
}
