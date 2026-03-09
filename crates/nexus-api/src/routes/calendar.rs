//! Calendar routes — 16-02: Calendar Integration.
//!
//! POST   /servers/:id/calendar                       — Create event
//! GET    /servers/:id/calendar                       — List events (date range)
//! GET    /servers/:id/calendar/:event_id             — Get event
//! PATCH  /servers/:id/calendar/:event_id             — Update event
//! DELETE /servers/:id/calendar/:event_id             — Delete event
//! POST   /calendar/:event_id/rsvp                    — RSVP to event
//! GET    /calendar/:event_id/rsvp                    — List RSVPs
//! DELETE /calendar/:event_id/rsvp                    — Remove RSVP
//! GET    /servers/:id/calendar.ics                   — ICS export

use axum::{
    extract::{Extension, Path, Query, State},
    http::header,
    middleware,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::snowflake;
use nexus_db::repository::calendar;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/servers/{server_id}/calendar",
            post(create_event).get(list_events),
        )
        .route(
            "/servers/{server_id}/calendar/{event_id}",
            get(get_event).patch(update_event).delete(delete_event),
        )
        .route(
            "/calendar/{event_id}/rsvp",
            post(rsvp).get(list_rsvps).delete(remove_rsvp),
        )
        .route("/servers/{server_id}/calendar.ics", get(export_ics))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateEventRequest {
    title: String,
    description: Option<String>,
    location: Option<String>,
    channel_id: Option<Uuid>,
    starts_at: String,
    ends_at: String,
    all_day: Option<bool>,
    rrule: Option<String>,
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateEventRequest {
    title: Option<String>,
    description: Option<String>,
    location: Option<String>,
    starts_at: Option<String>,
    ends_at: Option<String>,
    all_day: Option<bool>,
    rrule: Option<Option<String>>,
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RangeQuery {
    after: Option<String>,
    before: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RsvpRequest {
    status: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn create_event(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateEventRequest>,
) -> NexusResult<Json<nexus_common::models::collaboration::CalendarEvent>> {
    if body.title.is_empty() || body.title.len() > 200 {
        return Err(NexusError::Validation {
            message: "Title must be 1-200 characters".into(),
        });
    }

    let ev = calendar::create_event(
        &state.db.pool,
        snowflake::generate_id(),
        server_id,
        body.channel_id,
        ctx.user_id,
        &body.title,
        body.description.as_deref(),
        body.location.as_deref(),
        &body.starts_at,
        &body.ends_at,
        body.all_day.unwrap_or(false),
        body.rrule.as_deref(),
        body.color.as_deref(),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(ev))
}

async fn list_events(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<RangeQuery>,
) -> NexusResult<Json<Vec<nexus_common::models::collaboration::CalendarEvent>>> {
    let after = q.after.as_deref().unwrap_or("2000-01-01T00:00:00Z");
    let before = q.before.as_deref().unwrap_or("2100-01-01T00:00:00Z");
    let limit = q.limit.unwrap_or(200).min(500);

    calendar::list_events_by_server(&state.db.pool, server_id, after, before, limit)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn get_event(
    State(state): State<Arc<AppState>>,
    Path((_server_id, event_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<nexus_common::models::collaboration::CalendarEvent>> {
    calendar::get_event(&state.db.pool, event_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .map(Json)
        .ok_or(NexusError::NotFound {
            resource: "CalendarEvent".into(),
        })
}

async fn update_event(
    State(state): State<Arc<AppState>>,
    Path((_server_id, event_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateEventRequest>,
) -> NexusResult<Json<nexus_common::models::collaboration::CalendarEvent>> {
    let ev = calendar::update_event(
        &state.db.pool,
        event_id,
        body.title.as_deref(),
        body.description.as_deref(),
        body.location.as_deref(),
        body.starts_at.as_deref(),
        body.ends_at.as_deref(),
        body.all_day,
        body.rrule
            .as_ref()
            .map(|o| o.as_deref()),
        body.color.as_deref(),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(ev))
}

async fn delete_event(
    State(state): State<Arc<AppState>>,
    Path((_server_id, event_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let deleted = calendar::delete_event(&state.db.pool, event_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if !deleted {
        return Err(NexusError::NotFound {
            resource: "CalendarEvent".into(),
        });
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── RSVP ──────────────────────────────────────────────────────────────────────

async fn rsvp(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(event_id): Path<Uuid>,
    Json(body): Json<RsvpRequest>,
) -> NexusResult<Json<nexus_common::models::collaboration::CalendarRsvp>> {
    if !matches!(body.status.as_str(), "going" | "interested" | "declined") {
        return Err(NexusError::Validation {
            message: "Status must be going, interested, or declined".into(),
        });
    }
    calendar::upsert_rsvp(&state.db.pool, event_id, ctx.user_id, &body.status)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn list_rsvps(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<Uuid>,
) -> NexusResult<Json<Vec<nexus_common::models::collaboration::CalendarRsvp>>> {
    calendar::list_rsvps(&state.db.pool, event_id)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn remove_rsvp(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(event_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    calendar::delete_rsvp(&state.db.pool, event_id, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── ICS Export ────────────────────────────────────────────────────────────────

async fn export_ics(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<RangeQuery>,
) -> NexusResult<impl IntoResponse> {
    let after = q.after.as_deref().unwrap_or("2000-01-01T00:00:00Z");
    let before = q.before.as_deref().unwrap_or("2100-01-01T00:00:00Z");

    let ics = calendar::export_ics(&state.db.pool, server_id, after, before)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    Ok((
        [(header::CONTENT_TYPE, "text/calendar; charset=utf-8")],
        ics,
    ))
}
