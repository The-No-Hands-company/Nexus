//! Thread routes — create, manage, and participate in threads.
//!
//! POST   /channels/:id/threads                         — Start a thread
//! GET    /channels/:id/threads                         — List active threads
//! GET    /channels/:id/threads/archived                — List archived threads
//! GET    /channels/:id/threads/:thread_id              — Get thread info
//! PATCH  /channels/:id/threads/:thread_id              — Update thread settings
//! POST   /channels/:id/threads/:thread_id/members/@me  — Join thread
//! DELETE /channels/:id/threads/:thread_id/members/@me  — Leave thread
//! GET    /channels/:id/threads/:thread_id/members      — List members

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::rich::{CreateThreadRequest, Thread, ThreadRow, UpdateThreadRequest},
    validation::validate_request,
};
use nexus_db::repository::{channels, threads};
use nexus_common::gateway_event::GatewayEvent;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Thread CRUD
        .route(
            "/channels/{channel_id}/threads",
            get(list_active_threads).post(create_thread),
        )
        .route(
            "/channels/{channel_id}/threads/archived",
            get(list_archived_threads),
        )
        .route(
            "/channels/{channel_id}/threads/{thread_id}",
            get(get_thread).patch(update_thread),
        )
        // Thread membership
        .route(
            "/channels/{channel_id}/threads/{thread_id}/members/@me",
            post(join_thread).delete(leave_thread),
        )
        .route(
            "/channels/{channel_id}/threads/{thread_id}/members",
            get(list_thread_members),
        )
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

// ============================================================
// Response helpers
// ============================================================

fn thread_response(row: ThreadRow) -> Thread {
    Thread {
        id: row.channel_id,
        parent_channel_id: row.parent_channel_id.unwrap_or(row.channel_id),
        parent_message_id: row.parent_message_id,
        owner_id: row.owner_id,
        title: row.title,
        message_count: row.message_count,
        member_count: row.member_count,
        auto_archive_minutes: row.auto_archive_minutes,
        archived: row.archived,
        archived_at: row.archived_at,
        locked: row.locked,
        tags: row.tags,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

// ============================================================
// POST /channels/:channel_id/threads
// ============================================================

async fn create_thread(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateThreadRequest>,
) -> NexusResult<Json<Thread>> {
    validate_request(&body)?;

    // Verify parent channel exists
    let _channel = channels::find_by_id(&state.db.pool, channel_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Channel".into(),
        })?;

    // Verify the user is a member of the server / channel
    // (simplified: just check the channel row exists — full permission
    //  system would check role bitflags)

    // Validate auto-archive value
    let auto_archive = body.auto_archive_minutes.unwrap_or(1440);
    if ![60, 1440, 4320, 10080].contains(&auto_archive) {
        return Err(NexusError::Validation {
            message: "auto_archive_minutes must be 60, 1440, 4320, or 10080".into(),
        });
    }

    // Create a channel row of type 'thread' first
    let thread_channel_id = Uuid::new_v4();
    let tags = body.tags.unwrap_or_default();

    // Insert the channel record
    sqlx::query(
        r#"
        INSERT INTO channels (id, server_id, parent_id, name, channel_type, position, created_at, updated_at)
        SELECT $1, server_id, $2, $3, 'thread', 0, NOW(), NOW()
        FROM channels WHERE id = $2
        "#,
    )
    .bind(thread_channel_id.to_string())
    .bind(channel_id.to_string())
    .bind(&body.title)
    .execute(&state.db.pool)
    .await?;

    let row = threads::create_thread(
        &state.db.pool,
        thread_channel_id,
        channel_id,
        body.message_id,
        auth.user_id,
        &body.title,
        auto_archive,
        &tags,
    )
    .await?;

    // Auto-add the creator as a thread member
    let _ = threads::add_member(&state.db.pool, thread_channel_id, auth.user_id).await;

    let thread = thread_response(row);

    // Broadcast thread creation to connected clients
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "THREAD_CREATE".into(),
        data: serde_json::to_value(&thread).unwrap_or_default(),
        server_id: None,
        channel_id: Some(thread.id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(thread))
}

// ============================================================
// GET /channels/:channel_id/threads
// ============================================================

#[derive(Deserialize)]
struct ListThreadsParams {
    limit: Option<i64>,
}

async fn list_active_threads(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Query(params): Query<ListThreadsParams>,
) -> NexusResult<Json<Vec<Thread>>> {
    let _ = auth;
    let limit = params.limit.unwrap_or(50).min(100);

    let rows = threads::list_active(&state.db.pool, channel_id, limit).await?;
    let list: Vec<Thread> = rows.into_iter().map(thread_response).collect();
    Ok(Json(list))
}

// ============================================================
// GET /channels/:channel_id/threads/archived
// ============================================================

#[derive(Deserialize)]
struct ArchivedParams {
    limit: Option<i64>,
    before: Option<chrono::DateTime<chrono::Utc>>,
}

async fn list_archived_threads(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Query(params): Query<ArchivedParams>,
) -> NexusResult<Json<Vec<Thread>>> {
    let _ = auth;
    let limit = params.limit.unwrap_or(25).min(100);

    let rows = threads::list_archived(&state.db.pool, channel_id, limit, params.before).await?;
    let list: Vec<Thread> = rows.into_iter().map(thread_response).collect();
    Ok(Json(list))
}

// ============================================================
// GET /channels/:channel_id/threads/:thread_id
// ============================================================

async fn get_thread(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((_channel_id, thread_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<Thread>> {
    let _ = auth;
    let row = threads::find_by_id(&state.db.pool, thread_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Thread".into(),
        })?;
    Ok(Json(thread_response(row)))
}

// ============================================================
// PATCH /channels/:channel_id/threads/:thread_id
// ============================================================

async fn update_thread(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((_channel_id, thread_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateThreadRequest>,
) -> NexusResult<Json<Thread>> {
    validate_request(&body)?;

    // Must be thread owner or have MANAGE_THREADS permission (simplified: owner only)
    let existing = threads::find_by_id(&state.db.pool, thread_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Thread".into(),
        })?;

    if existing.owner_id != auth.user_id {
        return Err(NexusError::MissingPermission {
            permission: "MANAGE_THREADS".into(),
        });
    }

    let tags_slice = body.tags.as_deref();
    let row = threads::update_thread(
        &state.db.pool,
        thread_id,
        body.title.as_deref(),
        body.archived,
        body.locked,
        body.auto_archive_minutes,
        tags_slice,
    )
    .await?;

    let thread = thread_response(row);

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "THREAD_UPDATE".into(),
        data: serde_json::to_value(&thread).unwrap_or_default(),
        server_id: None,
        channel_id: Some(thread.id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(thread))
}

// ============================================================
// Thread membership
// ============================================================

async fn join_thread(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((_channel_id, thread_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    threads::add_member(&state.db.pool, thread_id, auth.user_id).await?;
    Ok(Json(serde_json::json!({ "joined": true })))
}

async fn leave_thread(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((_channel_id, thread_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let removed = threads::remove_member(&state.db.pool, thread_id, auth.user_id).await?;
    Ok(Json(serde_json::json!({ "left": removed })))
}

async fn list_thread_members(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((_channel_id, thread_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<Vec<Uuid>>> {
    let _ = auth;
    let members = threads::list_members(&state.db.pool, thread_id).await?;
    Ok(Json(members))
}

