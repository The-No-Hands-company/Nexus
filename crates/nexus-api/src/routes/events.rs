//! Server Events routes — scheduled events with RSVP.
//!
//! POST   /servers/{id}/events                     — create event
//! GET    /servers/{id}/events                     — list events
//! PATCH  /servers/{id}/events/{eid}               — update event
//! DELETE /servers/{id}/events/{eid}               — cancel event
//! PUT    /servers/{id}/events/{eid}/interested    — RSVP
//! DELETE /servers/{id}/events/{eid}/interested    — un-RSVP

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::{event_types, GatewayEvent},
    permissions::Permissions,
    snowflake,
};
use nexus_db::repository::{members, roles, servers};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/servers/{server_id}/events", get(list_events).post(create_event))
        .route(
            "/servers/{server_id}/events/{event_id}",
            get(get_event).patch(update_event).delete(cancel_event),
        )
        .route(
            "/servers/{server_id}/events/{event_id}/interested",
            put(rsvp).delete(un_rsvp),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ─────────────────────────────────────────────────────────────────────────────
// Models
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ServerEvent {
    pub id: Uuid,
    pub server_id: Uuid,
    pub creator_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub channel_id: Option<Uuid>,
    pub cover_image: Option<String>,
    pub status: String,
    pub interested_count: i64,
    pub is_interested: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateEventRequest {
    title: String,
    description: Option<String>,
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
    location: Option<String>,
    channel_id: Option<Uuid>,
    cover_image: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateEventRequest {
    title: Option<String>,
    description: Option<String>,
    starts_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
    location: Option<String>,
    cover_image: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListEventsParams {
    status: Option<String>,
    limit: Option<i64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Permission helper
// ─────────────────────────────────────────────────────────────────────────────

async fn require_manage_events(
    state: &AppState,
    user_id: Uuid,
    server_id: Uuid,
) -> NexusResult<()> {
    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Server".into() })?;
    if user_id == server.owner_id {
        return Ok(());
    }
    let member = members::find_member(&state.db.pool, user_id, server_id)
        .await?
        .ok_or(NexusError::Forbidden)?;
    let all_roles = roles::list_server_roles(&state.db.pool, server_id).await?;
    let base = all_roles.iter().find(|r| r.is_default)
        .map(|r| Permissions::from_bits_truncate(r.permissions))
        .unwrap_or_else(Permissions::empty);
    let effective = all_roles.iter()
        .filter(|r| !r.is_default && member.roles.contains(&r.id))
        .map(|r| Permissions::from_bits_truncate(r.permissions))
        .fold(base, |acc, rp| acc | rp);
    if effective.has(Permissions::MANAGE_EVENTS) || effective.has(Permissions::ADMINISTRATOR) {
        Ok(())
    } else {
        Err(NexusError::MissingPermission { permission: "MANAGE_EVENTS".into() })
    }
}

async fn fetch_event(state: &AppState, event_id: Uuid, requester_id: Uuid) -> NexusResult<ServerEvent> {
    let row = sqlx::query!(
        r#"
        SELECT
            e.id, e.server_id, e.creator_id, e.title, e.description,
            e.starts_at, e.ends_at, e.location, e.channel_id, e.cover_image,
            e.status, e.created_at, e.updated_at,
            COUNT(r.user_id) AS interested_count,
            BOOL_OR(r.user_id = $2) AS is_interested
        FROM server_events e
        LEFT JOIN server_event_rsvps r ON r.event_id = e.id
        WHERE e.id = $1
        GROUP BY e.id
        "#,
        event_id,
        requester_id,
    )
    .fetch_optional(&state.db.pool)
    .await?
    .ok_or(NexusError::NotFound { resource: "ServerEvent".into() })?;

    Ok(ServerEvent {
        id: row.id,
        server_id: row.server_id,
        creator_id: row.creator_id,
        title: row.title,
        description: row.description,
        starts_at: row.starts_at,
        ends_at: row.ends_at,
        location: row.location,
        channel_id: row.channel_id,
        cover_image: row.cover_image,
        status: row.status,
        interested_count: row.interested_count.unwrap_or(0),
        is_interested: row.is_interested.unwrap_or(false),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// POST /api/v1/servers/:server_id/events
async fn create_event(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateEventRequest>,
) -> NexusResult<Json<ServerEvent>> {
    require_manage_events(&state, auth.user_id, server_id).await?;

    if body.title.trim().is_empty() || body.title.len() > 100 {
        return Err(NexusError::Validation { message: "title must be 1–100 characters".into() });
    }
    if body.starts_at < Utc::now() {
        return Err(NexusError::Validation { message: "starts_at must be in the future".into() });
    }
    if let Some(ends_at) = body.ends_at {
        if ends_at <= body.starts_at {
            return Err(NexusError::Validation { message: "ends_at must be after starts_at".into() });
        }
    }

    let event_id = snowflake::generate_id();
    let now = Utc::now();

    sqlx::query!(
        r#"
        INSERT INTO server_events
            (id, server_id, creator_id, title, description, starts_at, ends_at,
             location, channel_id, cover_image, status, created_at, updated_at)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'scheduled',$11,$11)
        "#,
        event_id, server_id, auth.user_id,
        body.title.trim(), body.description.as_deref(),
        body.starts_at, body.ends_at, body.location.as_deref(),
        body.channel_id, body.cover_image.as_deref(), now,
    )
    .execute(&state.db.pool)
    .await?;

    let event = fetch_event(&state, event_id, auth.user_id).await?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::GUILD_SCHEDULED_EVENT_CREATE.into(),
        data: serde_json::to_value(&event).unwrap_or_default(),
        server_id: Some(server_id),
        channel_id: None,
        user_id: Some(auth.user_id),
    });

    Ok(Json(event))
}

/// GET /api/v1/servers/:server_id/events
async fn list_events(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Query(params): Query<ListEventsParams>,
) -> NexusResult<Json<Vec<ServerEvent>>> {
    // Verify membership
    let _ = members::find_member(&state.db.pool, auth.user_id, server_id)
        .await?
        .ok_or(NexusError::Forbidden)?;

    let status_filter = params.status.as_deref().unwrap_or("scheduled");
    let limit = params.limit.unwrap_or(50).min(100);

    let rows = sqlx::query!(
        r#"
        SELECT
            e.id, e.server_id, e.creator_id, e.title, e.description,
            e.starts_at, e.ends_at, e.location, e.channel_id, e.cover_image,
            e.status, e.created_at, e.updated_at,
            COUNT(r.user_id) AS interested_count,
            BOOL_OR(r.user_id = $3) AS is_interested
        FROM server_events e
        LEFT JOIN server_event_rsvps r ON r.event_id = e.id
        WHERE e.server_id = $1 AND e.status = $2
        GROUP BY e.id
        ORDER BY e.starts_at ASC
        LIMIT $4
        "#,
        server_id, status_filter, auth.user_id, limit,
    )
    .fetch_all(&state.db.pool)
    .await?;

    Ok(Json(rows.into_iter().map(|row| ServerEvent {
        id: row.id,
        server_id: row.server_id,
        creator_id: row.creator_id,
        title: row.title,
        description: row.description,
        starts_at: row.starts_at,
        ends_at: row.ends_at,
        location: row.location,
        channel_id: row.channel_id,
        cover_image: row.cover_image,
        status: row.status,
        interested_count: row.interested_count.unwrap_or(0),
        is_interested: row.is_interested.unwrap_or(false),
        created_at: row.created_at,
        updated_at: row.updated_at,
    }).collect()))
}

/// GET /api/v1/servers/:server_id/events/:event_id
async fn get_event(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((server_id, event_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<ServerEvent>> {
    let _ = members::find_member(&state.db.pool, auth.user_id, server_id)
        .await?
        .ok_or(NexusError::Forbidden)?;
    Ok(Json(fetch_event(&state, event_id, auth.user_id).await?))
}

/// PATCH /api/v1/servers/:server_id/events/:event_id
async fn update_event(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((server_id, event_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateEventRequest>,
) -> NexusResult<Json<ServerEvent>> {
    require_manage_events(&state, auth.user_id, server_id).await?;

    let now = Utc::now();
    sqlx::query!(
        r#"
        UPDATE server_events SET
            title       = COALESCE($2, title),
            description = COALESCE($3, description),
            starts_at   = COALESCE($4, starts_at),
            ends_at     = COALESCE($5, ends_at),
            location    = COALESCE($6, location),
            cover_image = COALESCE($7, cover_image),
            updated_at  = $8
        WHERE id = $1 AND server_id = $9
        "#,
        event_id,
        body.title.as_deref(),
        body.description.as_deref(),
        body.starts_at,
        body.ends_at,
        body.location.as_deref(),
        body.cover_image.as_deref(),
        now,
        server_id,
    )
    .execute(&state.db.pool)
    .await?;

    let event = fetch_event(&state, event_id, auth.user_id).await?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::GUILD_SCHEDULED_EVENT_UPDATE.into(),
        data: serde_json::to_value(&event).unwrap_or_default(),
        server_id: Some(server_id),
        channel_id: None,
        user_id: Some(auth.user_id),
    });

    Ok(Json(event))
}

/// DELETE /api/v1/servers/:server_id/events/:event_id
async fn cancel_event(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((server_id, event_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    require_manage_events(&state, auth.user_id, server_id).await?;

    sqlx::query!(
        "UPDATE server_events SET status = 'cancelled', updated_at = NOW() WHERE id = $1 AND server_id = $2",
        event_id, server_id,
    )
    .execute(&state.db.pool)
    .await?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::GUILD_SCHEDULED_EVENT_DELETE.into(),
        data: serde_json::json!({ "id": event_id, "server_id": server_id }),
        server_id: Some(server_id),
        channel_id: None,
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// PUT /api/v1/servers/:server_id/events/:event_id/interested — RSVP
async fn rsvp(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((server_id, event_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let _ = members::find_member(&state.db.pool, auth.user_id, server_id)
        .await?
        .ok_or(NexusError::Forbidden)?;

    sqlx::query!(
        "INSERT INTO server_event_rsvps (event_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        event_id, auth.user_id,
    )
    .execute(&state.db.pool)
    .await?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::GUILD_SCHEDULED_EVENT_USER_ADD.into(),
        data: serde_json::json!({ "event_id": event_id, "user_id": auth.user_id, "guild_id": server_id }),
        server_id: Some(server_id),
        channel_id: None,
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/v1/servers/:server_id/events/:event_id/interested — un-RSVP
async fn un_rsvp(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((server_id, event_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    sqlx::query!(
        "DELETE FROM server_event_rsvps WHERE event_id = $1 AND user_id = $2",
        event_id, auth.user_id,
    )
    .execute(&state.db.pool)
    .await?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::GUILD_SCHEDULED_EVENT_USER_REMOVE.into(),
        data: serde_json::json!({ "event_id": event_id, "user_id": auth.user_id, "guild_id": server_id }),
        server_id: Some(server_id),
        channel_id: None,
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "ok": true })))
}
