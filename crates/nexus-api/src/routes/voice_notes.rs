//! Voice & video note routes — 17-01: Voice & Video Notes.
//!
//! POST   /channels/:id/voice-notes          — Upload voice note
//! GET    /channels/:id/voice-notes          — List voice notes
//! GET    /voice-notes/:id                   — Get single voice note
//! PATCH  /voice-notes/:id/transcript        — Set transcript
//! DELETE /voice-notes/:id                   — Delete voice note
//!
//! POST   /channels/:id/video-notes          — Upload video note
//! GET    /channels/:id/video-notes          — List video notes
//! GET    /video-notes/:id                   — Get single video note
//! PATCH  /video-notes/:id/transcript        — Set transcript
//! DELETE /video-notes/:id                   — Delete video note

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{get, patch, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::snowflake;
use nexus_db::repository::voice_notes;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Voice notes
        .route(
            "/channels/{channel_id}/voice-notes",
            post(create_voice_note).get(list_voice_notes),
        )
        .route("/voice-notes/{id}", get(get_voice_note).delete(delete_voice_note))
        .route("/voice-notes/{id}/transcript", patch(set_voice_transcript))
        // Video notes
        .route(
            "/channels/{channel_id}/video-notes",
            post(create_video_note).get(list_video_notes),
        )
        .route("/video-notes/{id}", get(get_video_note).delete(delete_video_note))
        .route("/video-notes/{id}/transcript", patch(set_video_transcript))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request / Response ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateVoiceNoteRequest {
    storage_key: String,
    filename: String,
    content_type: String,
    size: i64,
    duration_ms: i32,
    waveform: Option<String>,
    message_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct CreateVideoNoteRequest {
    storage_key: String,
    filename: String,
    content_type: String,
    size: i64,
    duration_ms: i32,
    width: Option<i32>,
    height: Option<i32>,
    thumbnail_key: Option<String>,
    message_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct TranscriptRequest {
    transcript: String,
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

// ── Voice Note Handlers ───────────────────────────────────────────────────────

async fn create_voice_note(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateVoiceNoteRequest>,
) -> NexusResult<Json<nexus_common::models::multimedia::VoiceNote>> {
    if body.duration_ms <= 0 || body.duration_ms > 600_000 {
        return Err(NexusError::Validation {
            message: "Duration must be between 1ms and 10 minutes".into(),
        });
    }
    if body.size <= 0 {
        return Err(NexusError::Validation {
            message: "Size must be positive".into(),
        });
    }

    let note = voice_notes::create_voice_note(
        &state.db.pool,
        snowflake::generate_id(),
        channel_id,
        ctx.user_id,
        &body.storage_key,
        &body.filename,
        &body.content_type,
        body.size,
        body.duration_ms,
        body.waveform.as_deref(),
        body.message_id,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(note))
}

async fn list_voice_notes(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Query(q): Query<ListQuery>,
) -> NexusResult<Json<Vec<nexus_common::models::multimedia::VoiceNote>>> {
    let limit = q.limit.unwrap_or(50).min(100);
    let offset = q.offset.unwrap_or(0);
    voice_notes::list_voice_notes(&state.db.pool, channel_id, limit, offset)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn get_voice_note(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<nexus_common::models::multimedia::VoiceNote>> {
    voice_notes::find_voice_note(&state.db.pool, id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .map(Json)
        .ok_or(NexusError::NotFound {
            resource: "VoiceNote".into(),
        })
}

async fn set_voice_transcript(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<TranscriptRequest>,
) -> NexusResult<Json<nexus_common::models::multimedia::VoiceNote>> {
    if body.transcript.len() > 50_000 {
        return Err(NexusError::Validation {
            message: "Transcript too long (max 50000 chars)".into(),
        });
    }
    voice_notes::set_voice_note_transcript(&state.db.pool, id, &body.transcript)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn delete_voice_note(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let key = voice_notes::delete_voice_note(&state.db.pool, id, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if key.is_none() {
        return Err(NexusError::NotFound {
            resource: "VoiceNote".into(),
        });
    }
    // Storage cleanup would happen asynchronously
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Video Note Handlers ───────────────────────────────────────────────────────

async fn create_video_note(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateVideoNoteRequest>,
) -> NexusResult<Json<nexus_common::models::multimedia::VideoNote>> {
    if body.duration_ms <= 0 || body.duration_ms > 600_000 {
        return Err(NexusError::Validation {
            message: "Duration must be between 1ms and 10 minutes".into(),
        });
    }
    if body.size <= 0 {
        return Err(NexusError::Validation {
            message: "Size must be positive".into(),
        });
    }

    let note = voice_notes::create_video_note(
        &state.db.pool,
        snowflake::generate_id(),
        channel_id,
        ctx.user_id,
        &body.storage_key,
        &body.filename,
        &body.content_type,
        body.size,
        body.duration_ms,
        body.width,
        body.height,
        body.thumbnail_key.as_deref(),
        body.message_id,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(note))
}

async fn list_video_notes(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Query(q): Query<ListQuery>,
) -> NexusResult<Json<Vec<nexus_common::models::multimedia::VideoNote>>> {
    let limit = q.limit.unwrap_or(50).min(100);
    let offset = q.offset.unwrap_or(0);
    voice_notes::list_video_notes(&state.db.pool, channel_id, limit, offset)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn get_video_note(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<nexus_common::models::multimedia::VideoNote>> {
    voice_notes::find_video_note(&state.db.pool, id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .map(Json)
        .ok_or(NexusError::NotFound {
            resource: "VideoNote".into(),
        })
}

async fn set_video_transcript(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<TranscriptRequest>,
) -> NexusResult<Json<nexus_common::models::multimedia::VideoNote>> {
    if body.transcript.len() > 50_000 {
        return Err(NexusError::Validation {
            message: "Transcript too long (max 50000 chars)".into(),
        });
    }
    voice_notes::set_video_note_transcript(&state.db.pool, id, &body.transcript)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn delete_video_note(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let key = voice_notes::delete_video_note(&state.db.pool, id, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if key.is_none() {
        return Err(NexusError::NotFound {
            resource: "VideoNote".into(),
        });
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}
