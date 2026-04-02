//! Voice caption repository queries — live captions for voice channels.

use nexus_common::models::accessibility::VoiceCaption;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::VOICE_CAPTION_COLS;

/// Insert a new caption segment (interim or final).
pub async fn create_caption(
    pool: &AnyPool,
    id: Uuid,
    channel_id: Uuid,
    speaker_id: Uuid,
    text: &str,
    language: &str,
    is_final: bool,
) -> Result<VoiceCaption, sqlx::Error> {
    let q = format!(
        "INSERT INTO voice_captions (id, channel_id, speaker_id, text, language, is_final) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING {VOICE_CAPTION_COLS}"
    );
    sqlx::query_as::<_, VoiceCaption>(&q)
        .bind(id.to_string())
        .bind(channel_id.to_string())
        .bind(speaker_id.to_string())
        .bind(text)
        .bind(language)
        .bind(is_final)
        .fetch_one(pool)
        .await
}

/// Retrieve a caption by ID.
pub async fn find_by_id(
    pool: &AnyPool,
    id: Uuid,
) -> Result<Option<VoiceCaption>, sqlx::Error> {
    let q = format!(
        "SELECT {VOICE_CAPTION_COLS} FROM voice_captions \
         WHERE id = $1"
    );
    sqlx::query_as::<_, VoiceCaption>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

/// Finalise an interim caption (set is_final = true, update text, set ended_at).
pub async fn finalise_caption(
    pool: &AnyPool,
    id: Uuid,
    final_text: &str,
) -> Result<Option<VoiceCaption>, sqlx::Error> {
    let q = format!(
        "UPDATE voice_captions SET text = $2, is_final = true, ended_at = NOW() \
         WHERE id = $1 \
         RETURNING {VOICE_CAPTION_COLS}"
    );
    sqlx::query_as::<_, VoiceCaption>(&q)
        .bind(id.to_string())
        .bind(final_text)
        .fetch_optional(pool)
        .await
}

/// List recent captions for a voice channel (most recent first).
pub async fn list_channel_captions(
    pool: &AnyPool,
    channel_id: Uuid,
    limit: i64,
) -> Result<Vec<VoiceCaption>, sqlx::Error> {
    let q = format!(
        "SELECT {VOICE_CAPTION_COLS} FROM voice_captions \
         WHERE channel_id = $1 \
         ORDER BY created_at DESC LIMIT $2"
    );
    sqlx::query_as::<_, VoiceCaption>(&q)
        .bind(channel_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}

/// Purge captions older than the given number of hours.
pub async fn purge_old_captions(
    pool: &AnyPool,
    hours: i64,
) -> Result<u64, sqlx::Error> {
    let q = format!(
        "DELETE FROM voice_captions WHERE created_at < NOW() - interval '{hours} hours'"
    );
    let res = sqlx::query(&q).execute(pool).await?;
    Ok(res.rows_affected())
}
