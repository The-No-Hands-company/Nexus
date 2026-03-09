//! Repository: voice_quality — Phase 20-03.

use nexus_common::models::scalability::VoiceQualityLog;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::VOICE_QUALITY_LOG_COLS;

pub async fn create_voice_quality_log(
    pool: &AnyPool,
    id: Uuid,
    channel_id: Uuid,
    user_id: Uuid,
    sfu_node_id: Option<Uuid>,
    bitrate: i32,
    packet_loss: f32,
    jitter_ms: f32,
    latency_ms: i32,
    fec_enabled: bool,
    quality_score: f32,
) -> Result<VoiceQualityLog, sqlx::Error> {
    let q = format!(
        "INSERT INTO voice_quality_logs \
         (id, channel_id, user_id, sfu_node_id, bitrate, packet_loss, jitter_ms, \
          latency_ms, fec_enabled, quality_score) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         RETURNING {VOICE_QUALITY_LOG_COLS}"
    );
    sqlx::query_as::<_, VoiceQualityLog>(&q)
        .bind(id.to_string())
        .bind(channel_id.to_string())
        .bind(user_id.to_string())
        .bind(sfu_node_id.map(|u| u.to_string()))
        .bind(bitrate)
        .bind(packet_loss as f64)
        .bind(jitter_ms as f64)
        .bind(latency_ms)
        .bind(fec_enabled)
        .bind(quality_score as f64)
        .fetch_one(pool)
        .await
}

pub async fn list_voice_quality_logs(
    pool: &AnyPool,
    channel_id: Uuid,
    limit: i64,
) -> Result<Vec<VoiceQualityLog>, sqlx::Error> {
    let q = format!(
        "SELECT {VOICE_QUALITY_LOG_COLS} FROM voice_quality_logs \
         WHERE channel_id = $1 ORDER BY created_at DESC LIMIT $2"
    );
    sqlx::query_as::<_, VoiceQualityLog>(&q)
        .bind(channel_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}
