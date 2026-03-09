//! Story & ephemeral status routes — 17-02: Stories & Ephemeral Status.
//!
//! POST   /stories                 — Create a story
//! GET    /users/:id/stories       — List active stories by user
//! GET    /users/@me/stories/feed  — Friends story feed
//! GET    /stories/:id             — Get single story
//! DELETE /stories/:id             — Delete own story
//! POST   /stories/:id/view        — Record a view
//! GET    /stories/:id/viewers     — List viewers (author only)

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::snowflake;
use nexus_db::repository::{relationships, stories};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/stories", post(create_story))
        .route("/users/{user_id}/stories", get(list_user_stories))
        .route("/users/@me/stories/feed", get(story_feed))
        .route("/stories/{id}", get(get_story).delete(delete_story))
        .route("/stories/{id}/view", post(record_view))
        .route("/stories/{id}/viewers", get(list_viewers))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request / Response ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateStoryRequest {
    media_type: String,
    storage_key: Option<String>,
    text_content: Option<String>,
    text_style: Option<serde_json::Value>,
    visibility: Option<String>,
    duration_hours: Option<i32>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn create_story(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<CreateStoryRequest>,
) -> NexusResult<Json<nexus_common::models::multimedia::Story>> {
    if !matches!(body.media_type.as_str(), "image" | "video" | "text") {
        return Err(NexusError::Validation {
            message: "media_type must be image, video, or text".into(),
        });
    }
    if body.media_type == "text" && body.text_content.is_none() {
        return Err(NexusError::Validation {
            message: "text stories require text_content".into(),
        });
    }
    if body.media_type != "text" && body.storage_key.is_none() {
        return Err(NexusError::Validation {
            message: "image/video stories require storage_key".into(),
        });
    }
    let visibility = body.visibility.as_deref().unwrap_or("friends");
    if !matches!(visibility, "friends" | "server" | "everyone") {
        return Err(NexusError::Validation {
            message: "visibility must be friends, server, or everyone".into(),
        });
    }
    let duration_hours = body.duration_hours.unwrap_or(24).max(1).min(72);
    let text_style_str = body
        .text_style
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default());

    let story = stories::create_story(
        &state.db.pool,
        snowflake::generate_id(),
        ctx.user_id,
        &body.media_type,
        body.storage_key.as_deref(),
        body.text_content.as_deref(),
        text_style_str.as_deref(),
        visibility,
        duration_hours,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(story))
}

async fn list_user_stories(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> NexusResult<Json<Vec<nexus_common::models::multimedia::Story>>> {
    stories::list_active_by_user(&state.db.pool, user_id)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn story_feed(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<nexus_common::models::multimedia::Story>>> {
    // Get the user's friends list
    let rels = relationships::list_for_user(&state.db.pool, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    let friend_ids: Vec<Uuid> = rels
        .iter()
        .filter(|r| r.status == nexus_common::models::relationship::RelationshipStatus::Accepted)
        .map(|r| {
            if r.requester_id == ctx.user_id {
                r.addressee_id
            } else {
                r.requester_id
            }
        })
        .collect();
    // Include the user's own stories
    let mut all_ids = friend_ids;
    all_ids.push(ctx.user_id);

    stories::list_active_feed(&state.db.pool, &all_ids)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn get_story(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<nexus_common::models::multimedia::Story>> {
    stories::find_story(&state.db.pool, id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .map(Json)
        .ok_or(NexusError::NotFound {
            resource: "Story".into(),
        })
}

async fn delete_story(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let deleted = stories::delete_story(&state.db.pool, id, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if !deleted {
        return Err(NexusError::NotFound {
            resource: "Story".into(),
        });
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn record_view(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    stories::record_view(&state.db.pool, id, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn list_viewers(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<Vec<nexus_common::models::multimedia::StoryView>>> {
    // Only the story author can see viewers
    let story = stories::find_story(&state.db.pool, id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or(NexusError::NotFound {
            resource: "Story".into(),
        })?;
    if story.author_id != ctx.user_id {
        return Err(NexusError::Forbidden);
    }
    stories::list_viewers(&state.db.pool, id)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}
