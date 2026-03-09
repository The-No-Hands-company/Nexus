//! Translation & TTS routes — 18-03/04: Language, Voice Accessibility.
//!
//! GET    /messages/:id/translate?lang=xx  — Get or generate translation
//! POST   /messages/:id/translate          — Store a translation
//! GET    /messages/:id/translations       — List all translations for a message
//! POST   /messages/:id/tts               — Request TTS playback
//! PATCH  /tts-requests/:id               — Update TTS request status
//! GET    /users/@me/tts-queue            — List pending TTS requests

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::snowflake;
use nexus_db::repository::{translations, tts_requests};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Translation
        .route(
            "/messages/{id}/translate",
            get(get_translation).post(store_translation),
        )
        .route("/messages/{id}/translations", get(list_translations))
        // TTS
        .route("/messages/{id}/tts", post(request_tts))
        .route("/tts-requests/{id}", get(get_tts_status).patch(update_tts_status))
        .route("/users/@me/tts-queue", get(list_tts_queue))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request/Query types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TranslateQuery {
    lang: String,
}

#[derive(Debug, Deserialize)]
struct StoreTranslationRequest {
    source_language: String,
    target_language: String,
    translated_text: String,
}

#[derive(Debug, Deserialize)]
struct UpdateTtsStatusRequest {
    status: String,
}

#[derive(Debug, Deserialize)]
struct TtsRequest {
    channel_id: Uuid,
}

// ── Translation Handlers ──────────────────────────────────────────────────────

async fn get_translation(
    State(state): State<Arc<AppState>>,
    Path(message_id): Path<Uuid>,
    Query(q): Query<TranslateQuery>,
) -> NexusResult<Json<Option<nexus_common::models::accessibility::MessageTranslation>>> {
    if q.lang.len() < 2 || q.lang.len() > 5 {
        return Err(NexusError::Validation {
            message: "lang must be a valid ISO 639-1 code".into(),
        });
    }

    let translation = translations::get_translation(&state.db.pool, message_id, &q.lang)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(translation))
}

async fn store_translation(
    State(state): State<Arc<AppState>>,
    Path(message_id): Path<Uuid>,
    Json(body): Json<StoreTranslationRequest>,
) -> NexusResult<Json<nexus_common::models::accessibility::MessageTranslation>> {
    if body.translated_text.is_empty() || body.translated_text.len() > 10000 {
        return Err(NexusError::Validation {
            message: "translated_text must be 1-10000 characters".into(),
        });
    }

    let t = translations::upsert_translation(
        &state.db.pool,
        snowflake::generate_id(),
        message_id,
        &body.source_language,
        &body.target_language,
        &body.translated_text,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(t))
}

async fn list_translations(
    State(state): State<Arc<AppState>>,
    Path(message_id): Path<Uuid>,
) -> NexusResult<Json<Vec<nexus_common::models::accessibility::MessageTranslation>>> {
    translations::list_translations(&state.db.pool, message_id)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

// ── TTS Handlers ──────────────────────────────────────────────────────────────

async fn request_tts(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(message_id): Path<Uuid>,
    Json(body): Json<TtsRequest>,
) -> NexusResult<Json<nexus_common::models::accessibility::MessageTtsRequest>> {
    let req = tts_requests::create_tts_request(
        &state.db.pool,
        snowflake::generate_id(),
        ctx.user_id,
        message_id,
        body.channel_id,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(req))
}

async fn get_tts_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<nexus_common::models::accessibility::MessageTtsRequest>> {
    // Re-use the update path with no actual update to read
    // Actually just do a direct query
    let q = format!(
        "SELECT {} FROM message_tts_requests WHERE id = $1",
        nexus_db::select_cols::MESSAGE_TTS_REQUEST_COLS
    );
    sqlx::query_as::<_, nexus_common::models::accessibility::MessageTtsRequest>(&q)
        .bind(id.to_string())
        .fetch_optional(&state.db.pool)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .map(Json)
        .ok_or(NexusError::NotFound {
            resource: format!("tts_request {id}"),
        })
}

async fn update_tts_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTtsStatusRequest>,
) -> NexusResult<Json<nexus_common::models::accessibility::MessageTtsRequest>> {
    let valid = ["pending", "playing", "done"];
    if !valid.contains(&body.status.as_str()) {
        return Err(NexusError::Validation {
            message: format!("status must be one of: {}", valid.join(", ")),
        });
    }

    tts_requests::update_tts_status(&state.db.pool, id, &body.status)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .map(Json)
        .ok_or(NexusError::NotFound {
            resource: format!("tts_request {id}"),
        })
}

async fn list_tts_queue(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<nexus_common::models::accessibility::MessageTtsRequest>>> {
    tts_requests::list_pending(&state.db.pool, ctx.user_id)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}
