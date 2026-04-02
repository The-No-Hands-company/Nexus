//! File versioning & storage quota routes — 16-03.
//!
//! GET    /attachments/:id/versions                   — List versions
//! POST   /attachments/:id/versions                   — Upload new version
//! GET    /attachments/:id/versions/:ver               — Get specific version
//! DELETE /attachments/:id/versions/:ver_id            — Rollback (delete version)
//! GET    /servers/:id/storage                         — Get storage quota
//! PUT    /servers/:id/storage                         — Set max quota (owner)

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::get,
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::snowflake;
use nexus_db::repository::file_versions;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/attachments/{attachment_id}/versions",
            get(list_versions).post(create_version),
        )
        .route(
            "/attachments/{attachment_id}/versions/{version_number}",
            get(get_version),
        )
        .route(
            "/attachments/{attachment_id}/versions/{version_id}/delete",
            axum::routing::post(delete_version),
        )
        .route(
            "/servers/{server_id}/storage",
            get(get_quota).put(set_quota),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateVersionRequest {
    filename: String,
    content_type: Option<String>,
    size: i64,
    storage_key: String,
    sha256: Option<String>,
    comment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SetQuotaRequest {
    max_bytes: i64,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_versions(
    State(state): State<Arc<AppState>>,
    Path(attachment_id): Path<Uuid>,
) -> NexusResult<Json<Vec<nexus_common::models::collaboration::FileVersion>>> {
    file_versions::list_versions(&state.db.pool, attachment_id)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn create_version(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(attachment_id): Path<Uuid>,
    Json(body): Json<CreateVersionRequest>,
) -> NexusResult<Json<nexus_common::models::collaboration::FileVersion>> {
    if body.filename.is_empty() || body.filename.len() > 256 {
        return Err(NexusError::Validation {
            message: "Filename must be 1-256 characters".into(),
        });
    }

    let ver = file_versions::create_version(
        &state.db.pool,
        snowflake::generate_id(),
        attachment_id,
        ctx.user_id,
        &body.filename,
        body.content_type.as_deref(),
        body.size,
        &body.storage_key,
        body.sha256.as_deref(),
        body.comment.as_deref(),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(ver))
}

async fn get_version(
    State(state): State<Arc<AppState>>,
    Path((attachment_id, version_number)): Path<(Uuid, i32)>,
) -> NexusResult<Json<nexus_common::models::collaboration::FileVersion>> {
    file_versions::get_version(&state.db.pool, attachment_id, version_number)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .map(Json)
        .ok_or(NexusError::NotFound {
            resource: "FileVersion".into(),
        })
}

async fn delete_version(
    State(state): State<Arc<AppState>>,
    Path((_attachment_id, version_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let deleted = file_versions::delete_version(&state.db.pool, version_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if !deleted {
        return Err(NexusError::NotFound {
            resource: "FileVersion".into(),
        });
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Storage Quota ─────────────────────────────────────────────────────────────

async fn get_quota(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<nexus_common::models::collaboration::ServerStorageQuota>> {
    file_versions::get_quota(&state.db.pool, server_id)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn set_quota(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<SetQuotaRequest>,
) -> NexusResult<Json<nexus_common::models::collaboration::ServerStorageQuota>> {
    if body.max_bytes < 0 {
        return Err(NexusError::Validation {
            message: "max_bytes must be non-negative".into(),
        });
    }
    file_versions::update_quota_max(&state.db.pool, server_id, body.max_bytes)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}
