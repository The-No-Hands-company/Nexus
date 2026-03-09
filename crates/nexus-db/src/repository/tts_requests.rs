//! Message TTS request repository queries.

use nexus_common::models::accessibility::MessageTtsRequest;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::MESSAGE_TTS_REQUEST_COLS;

/// Create a TTS request for a message.
pub async fn create_tts_request(
    pool: &AnyPool,
    id: Uuid,
    user_id: Uuid,
    message_id: Uuid,
    channel_id: Uuid,
) -> Result<MessageTtsRequest, sqlx::Error> {
    let q = format!(
        "INSERT INTO message_tts_requests (id, user_id, message_id, channel_id) \
         VALUES ($1, $2, $3, $4) \
         RETURNING {MESSAGE_TTS_REQUEST_COLS}"
    );
    sqlx::query_as::<_, MessageTtsRequest>(&q)
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(message_id.to_string())
        .bind(channel_id.to_string())
        .fetch_one(pool)
        .await
}

/// Update TTS request status (pending → playing → done).
pub async fn update_tts_status(
    pool: &AnyPool,
    id: Uuid,
    status: &str,
) -> Result<Option<MessageTtsRequest>, sqlx::Error> {
    let q = format!(
        "UPDATE message_tts_requests SET status = $2 WHERE id = $1 \
         RETURNING {MESSAGE_TTS_REQUEST_COLS}"
    );
    sqlx::query_as::<_, MessageTtsRequest>(&q)
        .bind(id.to_string())
        .bind(status)
        .fetch_optional(pool)
        .await
}

/// List pending TTS requests for a user.
pub async fn list_pending(
    pool: &AnyPool,
    user_id: Uuid,
) -> Result<Vec<MessageTtsRequest>, sqlx::Error> {
    let q = format!(
        "SELECT {MESSAGE_TTS_REQUEST_COLS} FROM message_tts_requests \
         WHERE user_id = $1 AND status = 'pending' \
         ORDER BY created_at ASC"
    );
    sqlx::query_as::<_, MessageTtsRequest>(&q)
        .bind(user_id.to_string())
        .fetch_all(pool)
        .await
}

/// Purge completed/old TTS requests (older than 1 hour).
pub async fn purge_old_requests(pool: &AnyPool) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "DELETE FROM message_tts_requests WHERE status = 'done' OR created_at < NOW() - interval '1 hour'"
    )
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}
