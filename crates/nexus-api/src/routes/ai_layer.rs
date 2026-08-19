//! AI & intelligence layer routes — Phase 21.
//!
//! Search embeddings, AI suggestions, thread summaries, toxicity,
//! raid detection, voice transcripts, consent & audit.

use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    middleware,
    routing::get,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::models::Member;
use nexus_common::models::ai_intelligence::*;
use nexus_db::repository::{ai_intelligence, channels, members};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{AppState, middleware::AuthContext};

// ── Router ─────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // AI suggestions
        .route(
            "/channels/{channel_id}/ai-suggestions",
            get(list_ai_suggestions).post(create_ai_suggestion),
        )
        // Thread summaries
        .route(
            "/channels/{channel_id}/thread-summary",
            get(get_thread_summary).post(upsert_thread_summary),
        )
        // Toxicity
        .route(
            "/servers/{server_id}/toxicity-flags",
            get(list_flagged_toxicity),
        )
        // Raid detection
        .route("/servers/{server_id}/raids", get(list_raid_detections))
        // Voice transcripts
        .route(
            "/channels/{channel_id}/voice-transcripts",
            get(list_voice_transcripts),
        )
        // AI consent (per server)
        .route(
            "/servers/{server_id}/ai-consent",
            get(list_ai_consent).post(upsert_ai_consent),
        )
        // AI audit log (per server)
        .route("/servers/{server_id}/ai-audit-log", get(list_ai_audit_log))
        .route_layer(middleware::from_fn(
            crate::middleware::combined_auth_middleware,
        ))
}

// ── Request / Query ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LimitQuery {
    limit: Option<i64>,
}

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
    ensure_channel_server_membership(&state, ctx.user_id, channel_id).await?;

    let rows = ai_intelligence::list_ai_suggestions(
        &state.db.pool,
        ctx.user_id,
        channel_id,
        q.limit.unwrap_or(50),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_ai_suggestion(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateAiSuggestionReq>,
) -> NexusResult<Json<AiSuggestion>> {
    // ── Input validation ───────────────────────────────────────────────────
    if body.suggestion_type.len() > 64 {
        return Err(NexusError::Validation {
            message: "suggestion_type must be 64 characters or fewer".into(),
        });
    }
    if body.content.len() > 4000 {
        return Err(NexusError::Validation {
            message: "content must be 4000 characters or fewer".into(),
        });
    }
    if let Some(ref model) = body.model_name
        && model.len() > 64 {
            return Err(NexusError::Validation {
                message: "model_name must be 64 characters or fewer".into(),
            });
        }
    // Validate context_ids size to prevent abuse
    if let Some(ref ctx) = body.context_ids {
        let ctx_str = serde_json::to_string(ctx).unwrap_or_default();
        if ctx_str.len() > 100_000 {
            return Err(NexusError::Validation {
                message: "context_ids too large (max 100KB)".into(),
            });
        }
    }

    ensure_ai_generation_writes_enabled()?;

    let server_id = ensure_channel_server_membership(&state, ctx.user_id, channel_id).await?;
    ensure_ai_consent_enabled(&state, ctx.user_id, server_id, "ai_suggestions").await?;
    let model_name = body.model_name.as_deref().unwrap_or("default");

    let row = ai_intelligence::create_ai_suggestion(
        &state.db.pool,
        Uuid::new_v4(),
        ctx.user_id,
        channel_id,
        &body.suggestion_type,
        &body.content,
        &body.context_ids.unwrap_or(serde_json::json!([])),
        model_name,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    let _ = ai_intelligence::create_ai_audit_entry(
        &state.db.pool,
        Uuid::new_v4(),
        server_id,
        "ai_suggestions",
        "created",
        Some(ctx.user_id),
        &serde_json::json!({
            "channel_id": channel_id,
            "suggestion_id": row.id,
            "suggestion_type": &body.suggestion_type,
        }),
        Some(model_name),
        None,
    )
    .await;

    Ok(Json(row))
}

async fn get_thread_summary(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Option<ThreadSummary>>> {
    ensure_channel_server_membership(&state, ctx.user_id, channel_id).await?;

    // channel_id is used as thread_id for lookup
    let row = ai_intelligence::get_thread_summary(&state.db.pool, channel_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn upsert_thread_summary(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<UpsertThreadSummaryReq>,
) -> NexusResult<Json<ThreadSummary>> {
    ensure_ai_generation_writes_enabled()?;

    let server_id = ensure_channel_server_membership(&state, ctx.user_id, channel_id).await?;
    ensure_ai_consent_enabled(&state, ctx.user_id, server_id, "thread_summaries").await?;
    let model_name = body.model_name.as_deref().unwrap_or("default");
    let model_version = body.model_version.as_deref().unwrap_or("1.0");

    let row = ai_intelligence::upsert_thread_summary(
        &state.db.pool,
        Uuid::new_v4(),
        body.thread_id,
        channel_id,
        &body.summary,
        body.message_count.unwrap_or(0),
        model_name,
        model_version,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    let _ = ai_intelligence::create_ai_audit_entry(
        &state.db.pool,
        Uuid::new_v4(),
        server_id,
        "thread_summaries",
        "upserted",
        Some(ctx.user_id),
        &serde_json::json!({
            "channel_id": channel_id,
            "thread_id": body.thread_id,
            "summary_id": row.id,
            "message_count": body.message_count.unwrap_or(0),
        }),
        Some(model_name),
        Some(model_version),
    )
    .await;

    Ok(Json(row))
}

fn ensure_ai_generation_writes_enabled() -> NexusResult<()> {
    if nexus_common::config::get()
        .features
        .enable_ai_generation_writes
    {
        Ok(())
    } else {
        Err(NexusError::Validation {
            message: "AI generation write endpoints are disabled by server config".into(),
        })
    }
}

/// 501 until something actually scores toxicity.
///
/// `create_toxicity_score` exists in the repository and has no caller anywhere
/// in the workspace, so this table can never be written and the endpoint could
/// only ever answer `[]`. For a moderation surface that is the worst possible
/// lie: an empty list reads as "we scanned this server and found nothing
/// harmful", when nothing scanned anything. A caller acting on that would be
/// acting on a clean bill of health that was never issued.
///
/// Restore the body below when an inference backend lands; the repository
/// query it used is still there and still correct.
async fn list_flagged_toxicity(
    State(_state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(_server_id): Path<Uuid>,
    Query(_q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<ToxicityScore>>> {
    Err(NexusError::NotImplemented {
        feature: "Toxicity scoring".to_string(),
    })
}

/// 501 until something actually detects raids.
///
/// Same shape as toxicity above: `create_raid_detection` has no caller, so an
/// empty list would mean "no raid is happening" rather than "nobody is
/// watching". An operator checking this during an actual raid would be
/// reassured by silence.
async fn list_raid_detections(
    State(_state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(_server_id): Path<Uuid>,
    Query(_q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<RaidDetection>>> {
    Err(NexusError::NotImplemented {
        feature: "Raid detection".to_string(),
    })
}

/// 501 until speech-to-text exists.
///
/// `create_voice_transcript` has no caller, so this is an empty list forever.
/// Less dangerous than the two above — nobody is harmed by believing a call
/// was not transcribed — but equally untrue, and a client cannot tell "no
/// transcripts yet" from "transcription is not a thing here".
async fn list_voice_transcripts(
    State(_state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(_channel_id): Path<Uuid>,
    Query(_q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<VoiceTranscript>>> {
    Err(NexusError::NotImplemented {
        feature: "Voice transcription".to_string(),
    })
}

async fn list_ai_consent(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<AiConsent>>> {
    let member: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if member.is_none() {
        return Err(NexusError::Forbidden);
    }

    let rows = ai_intelligence::list_ai_consent(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn upsert_ai_consent(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<UpsertAiConsentReq>,
) -> NexusResult<Json<AiConsent>> {
    let member: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if member.is_none() {
        return Err(NexusError::Forbidden);
    }

    let row = ai_intelligence::upsert_ai_consent(
        &state.db.pool,
        ctx.user_id,
        server_id,
        &body.feature,
        body.enabled,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_ai_audit_log(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<AiAuditEntry>>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() {
        return Err(NexusError::Forbidden);
    }

    let rows =
        ai_intelligence::list_ai_audit_log(&state.db.pool, server_id, q.limit.unwrap_or(200))
            .await
            .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn ensure_channel_server_membership(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
) -> NexusResult<Uuid> {
    let channel = channels::find_by_id(&state.db.pool, channel_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or_else(|| NexusError::NotFound {
            resource: "channel".into(),
        })?;

    let server_id = channel.server_id.ok_or(NexusError::Forbidden)?;

    let member: Option<Member> = members::find_member(&state.db.pool, user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if member.is_none() {
        return Err(NexusError::Forbidden);
    }

    Ok(server_id)
}

async fn ensure_ai_consent_enabled(
    state: &AppState,
    user_id: Uuid,
    server_id: Uuid,
    feature: &str,
) -> NexusResult<()> {
    let consents = ai_intelligence::list_ai_consent(&state.db.pool, user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    let enabled = consents
        .iter()
        .any(|c| c.feature.eq_ignore_ascii_case(feature) && c.enabled);

    if enabled {
        return Ok(());
    }

    let _ = ai_intelligence::create_ai_audit_entry(
        &state.db.pool,
        Uuid::new_v4(),
        server_id,
        feature,
        "denied_consent_required",
        Some(user_id),
        &serde_json::json!({ "reason": "ai_consent_not_enabled" }),
        None,
        None,
    )
    .await;

    Err(NexusError::Forbidden)
}
