//! Voice caption routes — 18-04: Voice Accessibility.
//!
//! POST /channels/:id/captions        — Submit a caption segment
//! PATCH /captions/:id/finalise       — Finalise an interim caption
//! GET  /channels/:id/captions        — List recent captions for a channel

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::post,
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::snowflake;
use nexus_db::repository::voice_captions;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/channels/{id}/captions", post(submit_caption).get(list_captions))
        .route("/captions/{id}/finalise", post(finalise_caption))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request bodies ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SubmitCaptionRequest {
    text: String,
    language: Option<String>,
    is_final: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct FinaliseCaptionRequest {
    text: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn submit_caption(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<SubmitCaptionRequest>,
) -> NexusResult<Json<nexus_common::models::accessibility::VoiceCaption>> {
    if body.text.is_empty() || body.text.len() > 5000 {
        return Err(NexusError::Validation {
            message: "Caption text must be 1-5000 characters".into(),
        });
    }

    let lang = body.language.as_deref().unwrap_or("en");
    let is_final = body.is_final.unwrap_or(false);

    let caption = voice_captions::create_caption(
        &state.db.pool,
        snowflake::generate_id(),
        channel_id,
        ctx.user_id,
        &body.text,
        lang,
        is_final,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(caption))
}

async fn finalise_caption(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<FinaliseCaptionRequest>,
) -> NexusResult<Json<nexus_common::models::accessibility::VoiceCaption>> {
    if body.text.is_empty() || body.text.len() > 5000 {
        return Err(NexusError::Validation {
            message: "Caption text must be 1-5000 characters".into(),
        });
    }

    voice_captions::finalise_caption(&state.db.pool, id, &body.text)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .map(Json)
        .ok_or(NexusError::NotFound {
            resource: format!("caption {id}"),
        })
}

async fn list_captions(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Vec<nexus_common::models::accessibility::VoiceCaption>>> {
    let captions = voice_captions::list_channel_captions(&state.db.pool, channel_id, 100)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(captions))
}
