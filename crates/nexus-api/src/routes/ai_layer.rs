//! AI & intelligence layer routes — Phase 21.
//!
//! Search embeddings, AI suggestions, thread summaries, toxicity,
//! raid detection, voice transcripts, consent & audit.

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::get,
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::models::ai_intelligence::*;
use nexus_db::repository::{ai_intelligence, members};
use nexus_common::models::Member;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// ── Router ─────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // AI suggestions
        .route(
            "/channels/:channel_id/ai-suggestions",
            get(list_ai_suggestions).post(create_ai_suggestion),
        )
        // Thread summaries
        .route(
            "/channels/:channel_id/thread-summary",
            get(get_thread_summary).post(upsert_thread_summary),
        )
        // Toxicity
        .route(
            "/servers/:server_id/toxicity-flags",
            get(list_flagged_toxicity),
        )
        // Raid detection
        .route("/servers/:server_id/raids", get(list_raid_detections))
        // Voice transcripts
        .route(
            "/channels/:channel_id/voice-transcripts",
            get(list_voice_transcripts),
        )
        // AI consent (per server)
        .route(
            "/servers/:server_id/ai-consent",
            get(list_ai_consent).post(upsert_ai_consent),
        )
        // AI audit log (per server)
        .route("/servers/:server_id/ai-audit-log", get(list_ai_audit_log))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request / Query ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LimitQuery { limit: Option<i64> }

#[derive(Debug, Deserialize)]
struct CreateAiSuggestionReq {
    suggestion_type: String,
    content: String,
    context_ids: Option<serde_json::Value>,
    model_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpsertThreadSummaryReq {
    thread_id: Uuid,
    summary: String,
    message_count: Option<i32>,
    model_name: Option<String>,
    model_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpsertAiConsentReq {
    feature: String,
    enabled: bool,
}

// ── Handlers ──────────────────────────────────────────────────────────────

async fn list_ai_suggestions(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<AiSuggestion>>> {
    let rows = ai_intelligence::list_ai_suggestions(
        &state.db.pool, ctx.user_id, channel_id, q.limit.unwrap_or(50),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_ai_suggestion(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateAiSuggestionReq>,
) -> NexusResult<Json<AiSuggestion>> {
    let row = ai_intelligence::create_ai_suggestion(
        &state.db.pool, Uuid::new_v4(), ctx.user_id, channel_id,
        &body.suggestion_type, &body.content,
        &body.context_ids.unwrap_or(serde_json::json!([])),
        &body.model_name.unwrap_or_else(|| "default".into()),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn get_thread_summary(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Option<ThreadSummary>>> {
    // channel_id is used as thread_id for lookup
    let row = ai_intelligence::get_thread_summary(&state.db.pool, channel_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn upsert_thread_summary(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<UpsertThreadSummaryReq>,
) -> NexusResult<Json<ThreadSummary>> {
    let row = ai_intelligence::upsert_thread_summary(
        &state.db.pool, Uuid::new_v4(), body.thread_id, channel_id,
        &body.summary, body.message_count.unwrap_or(0),
        &body.model_name.unwrap_or_else(|| "default".into()),
        &body.model_version.unwrap_or_else(|| "1.0".into()),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_flagged_toxicity(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<ToxicityScore>>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let rows = ai_intelligence::list_flagged_toxicity(&state.db.pool, server_id, q.limit.unwrap_or(100))
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn list_raid_detections(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<RaidDetection>>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let rows = ai_intelligence::list_raid_detections(&state.db.pool, server_id, q.limit.unwrap_or(50))
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn list_voice_transcripts(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<VoiceTranscript>>> {
    let rows = ai_intelligence::list_voice_transcripts(&state.db.pool, channel_id, q.limit.unwrap_or(100))
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn list_ai_consent(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<AiConsent>>> {
    let rows = ai_intelligence::list_ai_consent(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn upsert_ai_consent(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<UpsertAiConsentReq>,
) -> NexusResult<Json<AiConsent>> {
    let row = ai_intelligence::upsert_ai_consent(
        &state.db.pool, ctx.user_id, server_id, &body.feature, body.enabled,
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_ai_audit_log(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<AiAuditEntry>>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let rows = ai_intelligence::list_ai_audit_log(&state.db.pool, server_id, q.limit.unwrap_or(200))
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}
