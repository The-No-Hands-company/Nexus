//! Drawing & annotation routes — 17-03: In-Chat Drawing & Annotation.
//!
//! POST   /channels/:id/drawings           — Create drawing
//! GET    /channels/:id/drawings           — List drawings in channel
//! GET    /channels/:id/whiteboards        — List active whiteboards
//! GET    /drawings/:id                    — Get single drawing
//! PATCH  /drawings/:id                    — Update drawing data
//! DELETE /drawings/:id                    — Delete drawing

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::snowflake;
use nexus_db::repository::drawings;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/channels/{channel_id}/drawings",
            post(create_drawing).get(list_drawings),
        )
        .route("/channels/{channel_id}/whiteboards", get(list_whiteboards))
        .route(
            "/drawings/{id}",
            get(get_drawing).patch(update_drawing).delete(delete_drawing),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request / Response ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateDrawingRequest {
    drawing_data: serde_json::Value,
    width: Option<i32>,
    height: Option<i32>,
    preview_key: Option<String>,
    message_id: Option<Uuid>,
    is_whiteboard: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpdateDrawingRequest {
    drawing_data: serde_json::Value,
    preview_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn create_drawing(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateDrawingRequest>,
) -> NexusResult<Json<nexus_common::models::multimedia::Drawing>> {
    let width = body.width.unwrap_or(800).max(1).min(4096);
    let height = body.height.unwrap_or(600).max(1).min(4096);
    let drawing_data_str = serde_json::to_string(&body.drawing_data)
        .map_err(|_| NexusError::Validation {
            message: "Invalid drawing data".into(),
        })?;

    let drawing = drawings::create_drawing(
        &state.db.pool,
        snowflake::generate_id(),
        channel_id,
        ctx.user_id,
        &drawing_data_str,
        width,
        height,
        body.preview_key.as_deref(),
        body.message_id,
        body.is_whiteboard.unwrap_or(false),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(drawing))
}

async fn list_drawings(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Query(q): Query<ListQuery>,
) -> NexusResult<Json<Vec<nexus_common::models::multimedia::Drawing>>> {
    let limit = q.limit.unwrap_or(50).min(100);
    let offset = q.offset.unwrap_or(0);
    drawings::list_by_channel(&state.db.pool, channel_id, limit, offset)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn list_whiteboards(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Vec<nexus_common::models::multimedia::Drawing>>> {
    drawings::list_whiteboards(&state.db.pool, channel_id)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn get_drawing(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<nexus_common::models::multimedia::Drawing>> {
    drawings::find_drawing(&state.db.pool, id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .map(Json)
        .ok_or(NexusError::NotFound {
            resource: "Drawing".into(),
        })
}

async fn update_drawing(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateDrawingRequest>,
) -> NexusResult<Json<nexus_common::models::multimedia::Drawing>> {
    let drawing_data_str = serde_json::to_string(&body.drawing_data)
        .map_err(|_| NexusError::Validation {
            message: "Invalid drawing data".into(),
        })?;

    drawings::update_drawing_data(
        &state.db.pool,
        id,
        &drawing_data_str,
        body.preview_key.as_deref(),
    )
    .await
    .map(Json)
    .map_err(|e| NexusError::Internal(e.into()))
}

async fn delete_drawing(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let deleted = drawings::delete_drawing(&state.db.pool, id, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if !deleted {
        return Err(NexusError::NotFound {
            resource: "Drawing".into(),
        });
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}
