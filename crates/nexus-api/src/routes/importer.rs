//! Import & migration routes — Phase 19-01.
//!
//! POST   /servers/:server_id/imports           — Start a data import job
//! GET    /servers/:server_id/imports           — List import jobs for a server
//! GET    /servers/:server_id/imports/:import_id — Get import job status
//! POST   /servers/:server_id/bulk-invite       — Send bulk email invitations

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::models::ecosystem::{BulkInvitation, ImportJob};
use nexus_db::repository::{bulk_invitations, import_jobs, members};
use nexus_common::models::Member;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/servers/:server_id/imports",
            post(create_import).get(list_imports),
        )
        .route(
            "/servers/:server_id/imports/:import_id",
            get(get_import),
        )
        .route(
            "/servers/:server_id/bulk-invite",
            post(create_bulk_invite),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request Bodies ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateImportRequest {
    /// discord | slack | matrix
    source_platform: String,
    /// Opaque JSON metadata (e.g. tokens, export path)
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct BulkInviteRequest {
    /// Array of email addresses
    emails: Vec<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn create_import(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateImportRequest>,
) -> NexusResult<Json<ImportJob>> {
    // Verify membership (admin-level action)
    let _member: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _member.is_none() { return Err(NexusError::Forbidden); }

    let valid_platforms = ["discord", "slack", "matrix"];
    if !valid_platforms.contains(&body.source_platform.as_str()) {
        return Err(NexusError::Validation {
            message: format!(
                "source_platform must be one of: {}",
                valid_platforms.join(", ")
            ),
        });
    }

    let id = Uuid::new_v4();
    let metadata = body.metadata.unwrap_or(serde_json::Value::Null);

    let job = import_jobs::create_import_job(
        &state.db.pool,
        id,
        server_id,
        ctx.user_id,
        &body.source_platform,
        &metadata,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(job))
}

async fn list_imports(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<ImportJob>>> {
    let _member: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _member.is_none() { return Err(NexusError::Forbidden); }

    let jobs = import_jobs::list_import_jobs(&state.db.pool, server_id, 100)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(jobs))
}

async fn get_import(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path((server_id, import_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<ImportJob>> {
    let _member: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _member.is_none() { return Err(NexusError::Forbidden); }

    let job = import_jobs::get_import_job(&state.db.pool, import_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or_else(|| NexusError::NotFound {
            resource: "import_job".into(),
        })?;

    Ok(Json(job))
}

async fn create_bulk_invite(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<BulkInviteRequest>,
) -> NexusResult<Json<BulkInvitation>> {
    let _member: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _member.is_none() { return Err(NexusError::Forbidden); }

    if body.emails.is_empty() || body.emails.len() > 500 {
        return Err(NexusError::Validation {
            message: "emails must contain 1 to 500 entries".into(),
        });
    }

    let id = Uuid::new_v4();
    let invite_code = format!("bulk-{}", Uuid::new_v4().simple());
    let emails_json = serde_json::to_value(&body.emails)
        .map_err(|e| NexusError::Internal(e.into()))?;
    let total = body.emails.len() as i32;

    let inv = bulk_invitations::create_bulk_invitation(
        &state.db.pool,
        id,
        server_id,
        ctx.user_id,
        &emails_json,
        total,
        Some(invite_code.as_str()),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(inv))
}
