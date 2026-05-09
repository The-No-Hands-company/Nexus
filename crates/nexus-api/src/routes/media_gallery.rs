//! Media gallery routes — 17-05: Media Gallery.
//!
//! GET    /media/browse                     — Browse media with filters
//! POST   /media/filters                    — Save a gallery filter preset
//! GET    /media/filters                    — List saved filter presets
//! DELETE /media/filters/:id                — Delete a saved filter preset

use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{delete, get, post},
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::snowflake;
use nexus_db::repository::media_gallery;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{AppState, middleware::AuthContext};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/media/browse", get(browse_media))
        .route("/media/filters", post(create_filter).get(list_filters))
        .route("/media/filters/{id}", delete(delete_filter))
        .route_layer(middleware::from_fn(
            crate::middleware::combined_auth_middleware,
        ))
}

// ── Request / Response ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct BrowseQuery {
    server_id: Option<Uuid>,
    channel_id: Option<Uuid>,
    #[serde(default)]
    media_types: Vec<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CreateFilterRequest {
    name: String,
    server_id: Option<Uuid>,
    channel_id: Option<Uuid>,
    media_types: Option<Vec<String>>,
    date_from: Option<String>,
    date_to: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn browse_media(
    State(state): State<Arc<AppState>>,
    Query(q): Query<BrowseQuery>,
) -> NexusResult<Json<Vec<nexus_common::models::rich::AttachmentRow>>> {
    let limit = q.limit.unwrap_or(50).min(100);
    let offset = q.offset.unwrap_or(0);
    let media_types = if q.media_types.is_empty() {
        vec![
            "image".to_string(),
            "video".to_string(),
            "audio".to_string(),
        ]
    } else {
        q.media_types
    };

    media_gallery::browse_media(
        &state.db.pool,
        q.server_id,
        q.channel_id,
        &media_types,
        q.date_from.as_deref(),
        q.date_to.as_deref(),
        limit,
        offset,
    )
    .await
    .map(Json)
    .map_err(|e| NexusError::Internal(e.into()))
}

async fn create_filter(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<CreateFilterRequest>,
) -> NexusResult<Json<nexus_common::models::multimedia::MediaGalleryFilter>> {
    if body.name.is_empty() || body.name.len() > 100 {
        return Err(NexusError::Validation {
            message: "Name must be 1-100 characters".into(),
        });
    }
    let media_types = body.media_types.unwrap_or_else(|| {
        vec![
            "image".to_string(),
            "video".to_string(),
            "audio".to_string(),
            "file".to_string(),
        ]
    });

    media_gallery::create_filter(
        &state.db.pool,
        snowflake::generate_id(),
        ctx.user_id,
        body.server_id,
        body.channel_id,
        &body.name,
        &media_types,
        body.date_from.as_deref(),
        body.date_to.as_deref(),
    )
    .await
    .map(Json)
    .map_err(|e| NexusError::Internal(e.into()))
}

async fn list_filters(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<nexus_common::models::multimedia::MediaGalleryFilter>>> {
    media_gallery::list_filters(&state.db.pool, ctx.user_id)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn delete_filter(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let deleted = media_gallery::delete_filter(&state.db.pool, id, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if !deleted {
        return Err(NexusError::NotFound {
            resource: "MediaGalleryFilter".into(),
        });
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}
