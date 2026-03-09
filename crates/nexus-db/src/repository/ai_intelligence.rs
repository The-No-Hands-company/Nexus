//! Repository: AI intelligence — Phase 21.

use nexus_common::models::ai_intelligence::*;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::*;

// ── 21-01: Semantic Search ────────────────────────────────────────────────

pub async fn create_search_embedding(
    pool: &AnyPool,
    id: Uuid,
    message_id: Uuid,
    channel_id: Uuid,
    embedding: Option<&[u8]>,
    model_name: &str,
    model_version: &str,
) -> Result<SearchEmbedding, sqlx::Error> {
    let q = format!(
        "INSERT INTO search_embeddings (id, message_id, channel_id, embedding, model_name, model_version) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (message_id) DO UPDATE SET embedding = $4, model_name = $5, model_version = $6 \
         RETURNING {SEARCH_EMBEDDING_COLS}"
    );
    sqlx::query_as::<_, SearchEmbedding>(&q)
        .bind(id.to_string())
        .bind(message_id.to_string())
        .bind(channel_id.to_string())
        .bind(embedding)
        .bind(model_name)
        .bind(model_version)
        .fetch_one(pool)
        .await
}

pub async fn log_search_query(
    pool: &AnyPool,
    id: Uuid,
    user_id: Uuid,
    raw_query: &str,
    parsed_filters: &serde_json::Value,
    result_count: i32,
    latency_ms: i32,
) -> Result<SearchQuery, sqlx::Error> {
    let q = format!(
        "INSERT INTO search_queries (id, user_id, raw_query, parsed_filters, result_count, latency_ms) \
         VALUES ($1, $2, $3, $4::jsonb, $5, $6) \
         RETURNING {SEARCH_QUERY_COLS}"
    );
    sqlx::query_as::<_, SearchQuery>(&q)
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(raw_query)
        .bind(parsed_filters.to_string())
        .bind(result_count)
        .bind(latency_ms)
        .fetch_one(pool)
        .await
}

// ── 21-02: Proactive Assists ──────────────────────────────────────────────

pub async fn create_ai_suggestion(
    pool: &AnyPool,
    id: Uuid,
    user_id: Uuid,
    channel_id: Uuid,
    suggestion_type: &str,
    content: &str,
    context_ids: &serde_json::Value,
    model_name: &str,
) -> Result<AiSuggestion, sqlx::Error> {
    let q = format!(
        "INSERT INTO ai_suggestions \
         (id, user_id, channel_id, suggestion_type, content, context_ids, model_name) \
         VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7) \
         RETURNING {AI_SUGGESTION_COLS}"
    );
    sqlx::query_as::<_, AiSuggestion>(&q)
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(channel_id.to_string())
        .bind(suggestion_type)
        .bind(content)
        .bind(context_ids.to_string())
        .bind(model_name)
        .fetch_one(pool)
        .await
}

pub async fn list_ai_suggestions(
    pool: &AnyPool,
    user_id: Uuid,
    channel_id: Uuid,
    limit: i64,
) -> Result<Vec<AiSuggestion>, sqlx::Error> {
    let q = format!(
        "SELECT {AI_SUGGESTION_COLS} FROM ai_suggestions \
         WHERE user_id = $1 AND channel_id = $2 ORDER BY created_at DESC LIMIT $3"
    );
    sqlx::query_as::<_, AiSuggestion>(&q)
        .bind(user_id.to_string())
        .bind(channel_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn upsert_thread_summary(
    pool: &AnyPool,
    id: Uuid,
    thread_id: Uuid,
    channel_id: Uuid,
    summary: &str,
    message_count: i32,
    model_name: &str,
    model_version: &str,
) -> Result<ThreadSummary, sqlx::Error> {
    let q = format!(
        "INSERT INTO thread_summaries \
         (id, thread_id, channel_id, summary, message_count, model_name, model_version) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (thread_id) DO UPDATE SET \
           summary = $4, message_count = $5, model_name = $6, model_version = $7, updated_at = now() \
         RETURNING {THREAD_SUMMARY_COLS}"
    );
    sqlx::query_as::<_, ThreadSummary>(&q)
        .bind(id.to_string())
        .bind(thread_id.to_string())
        .bind(channel_id.to_string())
        .bind(summary)
        .bind(message_count)
        .bind(model_name)
        .bind(model_version)
        .fetch_one(pool)
        .await
}

pub async fn get_thread_summary(
    pool: &AnyPool,
    thread_id: Uuid,
) -> Result<Option<ThreadSummary>, sqlx::Error> {
    let q = format!(
        "SELECT {THREAD_SUMMARY_COLS} FROM thread_summaries WHERE thread_id = $1"
    );
    sqlx::query_as::<_, ThreadSummary>(&q)
        .bind(thread_id.to_string())
        .fetch_optional(pool)
        .await
}

// ── 21-03: Moderation AI ─────────────────────────────────────────────────

pub async fn create_toxicity_score(
    pool: &AnyPool,
    id: Uuid,
    message_id: Uuid,
    server_id: Uuid,
    score: f32,
    categories: &serde_json::Value,
    model_name: &str,
    flagged: bool,
) -> Result<ToxicityScore, sqlx::Error> {
    let q = format!(
        "INSERT INTO toxicity_scores \
         (id, message_id, server_id, score, categories, model_name, flagged) \
         VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7) \
         RETURNING {TOXICITY_SCORE_COLS}"
    );
    sqlx::query_as::<_, ToxicityScore>(&q)
        .bind(id.to_string())
        .bind(message_id.to_string())
        .bind(server_id.to_string())
        .bind(score as f64)
        .bind(categories.to_string())
        .bind(model_name)
        .bind(flagged)
        .fetch_one(pool)
        .await
}

pub async fn list_flagged_toxicity(
    pool: &AnyPool,
    server_id: Uuid,
    limit: i64,
) -> Result<Vec<ToxicityScore>, sqlx::Error> {
    let q = format!(
        "SELECT {TOXICITY_SCORE_COLS} FROM toxicity_scores \
         WHERE server_id = $1 AND flagged = TRUE AND reviewed = FALSE \
         ORDER BY score DESC LIMIT $2"
    );
    sqlx::query_as::<_, ToxicityScore>(&q)
        .bind(server_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn create_raid_detection(
    pool: &AnyPool,
    id: Uuid,
    server_id: Uuid,
    detection_type: &str,
    severity: &str,
    details: &serde_json::Value,
    auto_actions: &serde_json::Value,
) -> Result<RaidDetection, sqlx::Error> {
    let q = format!(
        "INSERT INTO raid_detections \
         (id, server_id, detection_type, severity, details, auto_actions) \
         VALUES ($1, $2, $3, $4, $5::jsonb, $6::jsonb) \
         RETURNING {RAID_DETECTION_COLS}"
    );
    sqlx::query_as::<_, RaidDetection>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(detection_type)
        .bind(severity)
        .bind(details.to_string())
        .bind(auto_actions.to_string())
        .fetch_one(pool)
        .await
}

pub async fn list_raid_detections(
    pool: &AnyPool,
    server_id: Uuid,
    limit: i64,
) -> Result<Vec<RaidDetection>, sqlx::Error> {
    let q = format!(
        "SELECT {RAID_DETECTION_COLS} FROM raid_detections \
         WHERE server_id = $1 ORDER BY detected_at DESC LIMIT $2"
    );
    sqlx::query_as::<_, RaidDetection>(&q)
        .bind(server_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}

// ── 21-04: Voice AI ──────────────────────────────────────────────────────

pub async fn create_voice_transcript(
    pool: &AnyPool,
    id: Uuid,
    channel_id: Uuid,
    session_id: Option<Uuid>,
    speaker_id: Option<Uuid>,
    segment_start: f32,
    segment_end: f32,
    text: &str,
    language: &str,
    confidence: f32,
) -> Result<VoiceTranscript, sqlx::Error> {
    let q = format!(
        "INSERT INTO voice_transcripts \
         (id, channel_id, session_id, speaker_id, segment_start, segment_end, text, language, confidence) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING {VOICE_TRANSCRIPT_COLS}"
    );
    sqlx::query_as::<_, VoiceTranscript>(&q)
        .bind(id.to_string())
        .bind(channel_id.to_string())
        .bind(session_id.map(|u| u.to_string()))
        .bind(speaker_id.map(|u| u.to_string()))
        .bind(segment_start as f64)
        .bind(segment_end as f64)
        .bind(text)
        .bind(language)
        .bind(confidence as f64)
        .fetch_one(pool)
        .await
}

pub async fn list_voice_transcripts(
    pool: &AnyPool,
    channel_id: Uuid,
    limit: i64,
) -> Result<Vec<VoiceTranscript>, sqlx::Error> {
    let q = format!(
        "SELECT {VOICE_TRANSCRIPT_COLS} FROM voice_transcripts \
         WHERE channel_id = $1 ORDER BY created_at DESC LIMIT $2"
    );
    sqlx::query_as::<_, VoiceTranscript>(&q)
        .bind(channel_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}

// ── 21-05: AI Governance ─────────────────────────────────────────────────

pub async fn upsert_ai_consent(
    pool: &AnyPool,
    user_id: Uuid,
    server_id: Uuid,
    feature: &str,
    enabled: bool,
) -> Result<AiConsent, sqlx::Error> {
    let q = format!(
        "INSERT INTO ai_consent (user_id, server_id, feature, enabled) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (user_id, server_id, feature) DO UPDATE SET enabled = $4, updated_at = now() \
         RETURNING {AI_CONSENT_COLS}"
    );
    sqlx::query_as::<_, AiConsent>(&q)
        .bind(user_id.to_string())
        .bind(server_id.to_string())
        .bind(feature)
        .bind(enabled)
        .fetch_one(pool)
        .await
}

pub async fn list_ai_consent(
    pool: &AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<Vec<AiConsent>, sqlx::Error> {
    let q = format!(
        "SELECT {AI_CONSENT_COLS} FROM ai_consent WHERE user_id = $1 AND server_id = $2"
    );
    sqlx::query_as::<_, AiConsent>(&q)
        .bind(user_id.to_string())
        .bind(server_id.to_string())
        .fetch_all(pool)
        .await
}

pub async fn create_ai_audit_entry(
    pool: &AnyPool,
    id: Uuid,
    server_id: Uuid,
    feature: &str,
    action: &str,
    actor_id: Option<Uuid>,
    details: &serde_json::Value,
    model_name: Option<&str>,
    model_version: Option<&str>,
) -> Result<AiAuditEntry, sqlx::Error> {
    let q = format!(
        "INSERT INTO ai_audit_log \
         (id, server_id, feature, action, actor_id, details, model_name, model_version) \
         VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8) \
         RETURNING {AI_AUDIT_ENTRY_COLS}"
    );
    sqlx::query_as::<_, AiAuditEntry>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(feature)
        .bind(action)
        .bind(actor_id.map(|u| u.to_string()))
        .bind(details.to_string())
        .bind(model_name)
        .bind(model_version)
        .fetch_one(pool)
        .await
}

pub async fn list_ai_audit_log(
    pool: &AnyPool,
    server_id: Uuid,
    limit: i64,
) -> Result<Vec<AiAuditEntry>, sqlx::Error> {
    let q = format!(
        "SELECT {AI_AUDIT_ENTRY_COLS} FROM ai_audit_log \
         WHERE server_id = $1 ORDER BY created_at DESC LIMIT $2"
    );
    sqlx::query_as::<_, AiAuditEntry>(&q)
        .bind(server_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}
