//! Admin analytics dashboard routes — Phase 19-03.
//!
//! GET  /servers/:server_id/analytics-snapshots  — List server analytics snapshots
//! POST /servers/:server_id/analytics-snapshots  — Upsert a daily snapshot (system/admin)

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::get,
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::models::ecosystem::ServerAnalyticsSnapshot;
use nexus_db::repository::{analytics_snapshots, members};
use nexus_common::models::Member;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/servers/:server_id/analytics-snapshots",
            get(list_snapshots).post(upsert_snapshot),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request / Query ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SnapshotQuery {
    /// Number of days to look back (default 30, max 365)
    days: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct UpsertSnapshotRequest {
    period_date: String, // YYYY-MM-DD
    messages_count: Option<i32>,
    active_members: Option<i32>,
    new_members: Option<i32>,
    left_members: Option<i32>,
    voice_minutes: Option<i32>,
    reports_resolved: Option<i32>,
    bans_issued: Option<i32>,
    filters_triggered: Option<i32>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_snapshots(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<SnapshotQuery>,
) -> NexusResult<Json<Vec<ServerAnalyticsSnapshot>>> {
    // Verify membership
    let _member: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _member.is_none() { return Err(NexusError::Forbidden); }

    let days = q.days.unwrap_or(30).min(365);

    let snapshots = analytics_snapshots::list_snapshots(&state.db.pool, server_id, days)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(snapshots))
}

async fn upsert_snapshot(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<UpsertSnapshotRequest>,
) -> NexusResult<Json<ServerAnalyticsSnapshot>> {
    // Verify membership
    let _member: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _member.is_none() { return Err(NexusError::Forbidden); }

    // Basic date format validation
    if chrono::NaiveDate::parse_from_str(&body.period_date, "%Y-%m-%d").is_err() {
        return Err(NexusError::Validation {
            message: "period_date must be a valid date in YYYY-MM-DD format".into(),
        });
    }

    let id = Uuid::new_v4();
    let snap = analytics_snapshots::upsert_snapshot(
        &state.db.pool,
        id,
        server_id,
        &body.period_date,
        body.messages_count.unwrap_or(0),
        body.active_members.unwrap_or(0),
        body.new_members.unwrap_or(0),
        body.left_members.unwrap_or(0),
        body.voice_minutes.unwrap_or(0),
        body.reports_resolved.unwrap_or(0),
        body.bans_issued.unwrap_or(0),
        body.filters_triggered.unwrap_or(0),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(snap))
}
