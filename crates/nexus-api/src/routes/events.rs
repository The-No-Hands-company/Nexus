//! Server Events routes — scheduled events with RSVP.
//!
//! POST   /servers/{id}/events                     — create event
//! GET    /servers/{id}/events                     — list events
//! PATCH  /servers/{id}/events/{eid}               — update event
//! DELETE /servers/{id}/events/{eid}               — cancel event
//! PUT    /servers/{id}/events/{eid}/interested    — RSVP
//! DELETE /servers/{id}/events/{eid}/interested    — un-RSVP

use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{get, put},
};
use chrono::{DateTime, Utc};
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::{GatewayEvent, event_types},
    permissions::Permissions,
    snowflake,
};
use nexus_db::repository::{members, roles, servers};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::{AppState, middleware::AuthContext};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/servers/{server_id}/events",
            get(list_events).post(create_event),
        )
        .route(
            "/servers/{server_id}/events/{event_id}",
            get(get_event).patch(update_event).delete(cancel_event),
        )
        .route(
            "/servers/{server_id}/events/{event_id}/interested",
            put(rsvp).delete(un_rsvp),
        )
        .route_layer(middleware::from_fn(
            crate::middleware::combined_auth_middleware,
        ))
}

// —————————————————————————————————————————————————————————————————————————————
// Models
// —————————————————————————————————————————————————————————————————————————————

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

// —————————————————————————————————————————————————————————————————————————————
// DateTime helper
// —————————————————————————————————————————————————————————————————————————————

/// Parse a PostgreSQL timestamptz ::text value into DateTime<Utc>.
/// Handles "2024-01-15 10:30:00+00", "2024-01-15 10:30:00.123456+00", RFC3339.
fn parse_pg_dt(s: &str) -> Option<DateTime<Utc>> {
    // RFC3339 first (e.g. "2024-01-15T10:30:00Z")
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    // PostgreSQL outputs "+00" suffix — normalize to "+0000" for chrono %z
    let normalized = if s.ends_with("+00") {
        format!("{}00", s)
    } else {
        s.to_string()
    };
    if let Ok(dt) = DateTime::parse_from_str(&normalized, "%Y-%m-%d %H:%M:%S%.f%z") {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = DateTime::parse_from_str(&normalized, "%Y-%m-%d %H:%M:%S%z") {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    None
}

// —————————————————————————————————————————————————————————————————————————————
// Permission helper
// —————————————————————————————————————————————————————————————————————————————

async fn require_manage_events(
    state: &AppState,
    user_id: Uuid,
    server_id: Uuid,
) -> NexusResult<()> {
    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Server".into(),
        })?;
    if user_id == server.owner_id {
        return Ok(());
    }
    let member = members::find_member(&state.db.pool, user_id, server_id)
        .await?
        .ok_or(NexusError::Forbidden)?;
    let all_roles = roles::list_server_roles(&state.db.pool, server_id).await?;
    let base = all_roles
        .iter()
        .find(|r| r.is_default)
        .map(|r| Permissions::from_bits_truncate(r.permissions))
        .unwrap_or_else(Permissions::empty);
    let effective = all_roles
        .iter()
        .filter(|r| !r.is_default && member.roles.contains(&r.id))
        .map(|r| Permissions::from_bits_truncate(r.permissions))
        .fold(base, |acc, rp| acc | rp);
    if effective.has(Permissions::MANAGE_EVENTS) || effective.has(Permissions::ADMINISTRATOR) {
        Ok(())
    } else {
        Err(NexusError::MissingPermission {
            permission: "MANAGE_EVENTS".into(),
        })
    }
}

async fn fetch_event(
    state: &AppState,
    event_id: Uuid,
    requester_id: Uuid,
) -> NexusResult<ServerEvent> {
    let row = sqlx::query(
        r#"
        SELECT
            e.id::text            AS id,
            e.server_id::text     AS server_id,
            e.creator_id::text    AS creator_id,
            e.title,
            e.description,
            e.starts_at::text     AS starts_at,
            e.ends_at::text       AS ends_at,
            e.location,
            e.channel_id::text    AS channel_id,
            e.cover_image,
            e.status,
            e.created_at::text    AS created_at,
            e.updated_at::text    AS updated_at,
            COUNT(r.user_id)      AS interested_count,
            BOOL_OR(r.user_id = $2::uuid) AS is_interested
        FROM server_events e
        LEFT JOIN server_event_rsvps r ON r.event_id = e.id
        WHERE e.id = $1::uuid
        GROUP BY e.id
        "#,
    )
    .bind(event_id.to_string())
    .bind(requester_id.to_string())
    .fetch_optional(&state.db.pool)
    .await
    .map_err(NexusError::Database)?
    .ok_or(NexusError::NotFound {
        resource: "ServerEvent".into(),
    })?;

    Ok(ServerEvent {
        id: row
            .try_get::<String, _>("id")
            .unwrap_or_default()
            .parse()
            .unwrap_or(event_id),
        server_id: row
            .try_get::<String, _>("server_id")
            .unwrap_or_default()
            .parse()
            .unwrap_or_default(),
        creator_id: row
            .try_get::<String, _>("creator_id")
            .unwrap_or_default()
            .parse()
            .unwrap_or_default(),
        title: row.try_get("title").unwrap_or_default(),
        description: row
            .try_get::<Option<String>, _>("description")
            .unwrap_or(None),
        starts_at: row
            .try_get::<String, _>("starts_at")
            .ok()
            .as_deref()
            .and_then(parse_pg_dt)
            .unwrap_or_else(Utc::now),
        ends_at: row
            .try_get::<Option<String>, _>("ends_at")
            .ok()
            .flatten()
            .as_deref()
            .and_then(parse_pg_dt),
        location: row.try_get::<Option<String>, _>("location").unwrap_or(None),
        channel_id: row
            .try_get::<Option<String>, _>("channel_id")
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok()),
        cover_image: row
            .try_get::<Option<String>, _>("cover_image")
            .unwrap_or(None),
        status: row.try_get("status").unwrap_or_else(|_| "scheduled".into()),
        interested_count: row.try_get::<i64, _>("interested_count").unwrap_or(0),
        is_interested: row
            .try_get::<Option<bool>, _>("is_interested")
            .ok()
            .flatten()
            .unwrap_or(false),
        created_at: row
            .try_get::<String, _>("created_at")
            .ok()
            .as_deref()
            .and_then(parse_pg_dt)
            .unwrap_or_else(Utc::now),
        updated_at: row
            .try_get::<String, _>("updated_at")
            .ok()
            .as_deref()
            .and_then(parse_pg_dt)
            .unwrap_or_else(Utc::now),
    })
}

// —————————————————————————————————————————————————————————————————————————————
// Handlers
// —————————————————————————————————————————————————————————————————————————————

/// POST /api/v1/servers/:server_id/events
async fn create_event(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateEventRequest>,
) -> NexusResult<Json<ServerEvent>> {
    require_manage_events(&state, auth.user_id, server_id).await?;

    if body.title.trim().is_empty() || body.title.len() > 100 {
        return Err(NexusError::Validation {
            message: "title must be 1–100 characters".into(),
        });
    }
    if body.starts_at < Utc::now() {
        return Err(NexusError::Validation {
            message: "starts_at must be in the future".into(),
        });
    }
    if let Some(ends_at) = body.ends_at
        && ends_at <= body.starts_at {
            return Err(NexusError::Validation {
                message: "ends_at must be after starts_at".into(),
            });
        }

    let event_id = snowflake::generate_id();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO server_events
            (id, server_id, creator_id, title, description, starts_at, ends_at,
             location, channel_id, cover_image, status, created_at, updated_at)
        VALUES (
            $1::uuid, $2::uuid, $3::uuid, $4, $5,
            $6::timestamptz, $7::timestamptz, $8, $9::uuid, $10,
            'scheduled', $11::timestamptz, $11::timestamptz
        )
        "#,
    )
    .bind(event_id.to_string())
    .bind(server_id.to_string())
    .bind(auth.user_id.to_string())
    .bind(body.title.trim().to_string())
    .bind(body.description.clone())
    .bind(body.starts_at.to_rfc3339())
    .bind(body.ends_at.map(|d| d.to_rfc3339()))
    .bind(body.location.clone())
    .bind(body.channel_id.map(|id| id.to_string()))
    .bind(body.cover_image.clone())
    .bind(now.to_rfc3339())
    .execute(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

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

    let status_filter = params.status.clone().unwrap_or_else(|| "scheduled".into());
    let limit = params.limit.unwrap_or(50).min(100);

    let rows = sqlx::query(
        r#"
        SELECT
            e.id::text            AS id,
            e.server_id::text     AS server_id,
            e.creator_id::text    AS creator_id,
            e.title,
            e.description,
            e.starts_at::text     AS starts_at,
            e.ends_at::text       AS ends_at,
            e.location,
            e.channel_id::text    AS channel_id,
            e.cover_image,
            e.status,
            e.created_at::text    AS created_at,
            e.updated_at::text    AS updated_at,
            COUNT(r.user_id)      AS interested_count,
            BOOL_OR(r.user_id = $3::uuid) AS is_interested
        FROM server_events e
        LEFT JOIN server_event_rsvps r ON r.event_id = e.id
        WHERE e.server_id = $1::uuid AND e.status = $2
        GROUP BY e.id
        ORDER BY e.starts_at ASC
        LIMIT $4
        "#,
    )
    .bind(server_id.to_string())
    .bind(status_filter)
    .bind(auth.user_id.to_string())
    .bind(limit)
    .fetch_all(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    let events: Vec<ServerEvent> = rows
        .iter()
        .map(|row| ServerEvent {
            id: row
                .try_get::<String, _>("id")
                .unwrap_or_default()
                .parse()
                .unwrap_or_default(),
            server_id: row
                .try_get::<String, _>("server_id")
                .unwrap_or_default()
                .parse()
                .unwrap_or_default(),
            creator_id: row
                .try_get::<String, _>("creator_id")
                .unwrap_or_default()
                .parse()
                .unwrap_or_default(),
            title: row.try_get("title").unwrap_or_default(),
            description: row
                .try_get::<Option<String>, _>("description")
                .unwrap_or(None),
            starts_at: row
                .try_get::<String, _>("starts_at")
                .ok()
                .as_deref()
                .and_then(parse_pg_dt)
                .unwrap_or_else(Utc::now),
            ends_at: row
                .try_get::<Option<String>, _>("ends_at")
                .ok()
                .flatten()
                .as_deref()
                .and_then(parse_pg_dt),
            location: row.try_get::<Option<String>, _>("location").unwrap_or(None),
            channel_id: row
                .try_get::<Option<String>, _>("channel_id")
                .ok()
                .flatten()
                .and_then(|s| s.parse().ok()),
            cover_image: row
                .try_get::<Option<String>, _>("cover_image")
                .unwrap_or(None),
            status: row.try_get("status").unwrap_or_else(|_| "scheduled".into()),
            interested_count: row.try_get::<i64, _>("interested_count").unwrap_or(0),
            is_interested: row
                .try_get::<Option<bool>, _>("is_interested")
                .ok()
                .flatten()
                .unwrap_or(false),
            created_at: row
                .try_get::<String, _>("created_at")
                .ok()
                .as_deref()
                .and_then(parse_pg_dt)
                .unwrap_or_else(Utc::now),
            updated_at: row
                .try_get::<String, _>("updated_at")
                .ok()
                .as_deref()
                .and_then(parse_pg_dt)
                .unwrap_or_else(Utc::now),
        })
        .collect();

    Ok(Json(events))
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
    sqlx::query(
        r#"
        UPDATE server_events SET
            title       = COALESCE($2, title),
            description = COALESCE($3, description),
            starts_at   = COALESCE($4::timestamptz, starts_at),
            ends_at     = COALESCE($5::timestamptz, ends_at),
            location    = COALESCE($6, location),
            cover_image = COALESCE($7, cover_image),
            updated_at  = $8::timestamptz
        WHERE id = $1::uuid AND server_id = $9::uuid
        "#,
    )
    .bind(event_id.to_string())
    .bind(body.title.clone())
    .bind(body.description.clone())
    .bind(body.starts_at.map(|d| d.to_rfc3339()))
    .bind(body.ends_at.map(|d| d.to_rfc3339()))
    .bind(body.location.clone())
    .bind(body.cover_image.clone())
    .bind(now.to_rfc3339())
    .bind(server_id.to_string())
    .execute(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

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

    sqlx::query(
        "UPDATE server_events SET status = 'cancelled', updated_at = NOW() WHERE id = $1::uuid AND server_id = $2::uuid",
    )
    .bind(event_id.to_string())
    .bind(server_id.to_string())
    .execute(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

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

    sqlx::query(
        "INSERT INTO server_event_rsvps (event_id, user_id) VALUES ($1::uuid, $2::uuid) ON CONFLICT DO NOTHING",
    )
    .bind(event_id.to_string())
    .bind(auth.user_id.to_string())
    .execute(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

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
    sqlx::query("DELETE FROM server_event_rsvps WHERE event_id = $1::uuid AND user_id = $2::uuid")
        .bind(event_id.to_string())
        .bind(auth.user_id.to_string())
        .execute(&state.db.pool)
        .await
        .map_err(NexusError::Database)?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::GUILD_SCHEDULED_EVENT_USER_REMOVE.into(),
        data: serde_json::json!({ "event_id": event_id, "user_id": auth.user_id, "guild_id": server_id }),
        server_id: Some(server_id),
        channel_id: None,
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "ok": true })))
}
