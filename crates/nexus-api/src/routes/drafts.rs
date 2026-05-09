//! Draft message routes — upsert, fetch, delete.
//!
//! Each user has at most one draft per channel.  Drafts are auto-saved by
//! the client on a debounce; the server stores the latest content.
//! The draft is cleared automatically when a message is sent (client calls DELETE).
//!
//! PUT    /channels/:id/draft   — Upsert draft (create or replace)
//! GET    /channels/:id/draft   — Fetch current draft
//! DELETE /channels/:id/draft   — Clear draft

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    middleware,
    routing::get,
};
use chrono::{DateTime, Utc};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::snowflake;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{AppState, middleware::AuthContext};

// ============================================================
// Router
// ============================================================

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/channels/{channel_id}/draft",
            get(get_draft).put(upsert_draft).delete(delete_draft),
        )
        .route_layer(middleware::from_fn(
            crate::middleware::combined_auth_middleware,
        ))
}

// ============================================================
// Models
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    pub id: Uuid,
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub content: String,
    pub attachments: Vec<serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct UpsertDraftRequest {
    content: String,
    attachments: Option<Vec<serde_json::Value>>,
}

// ============================================================
// DB row helpers
// ============================================================

#[derive(sqlx::FromRow)]
struct DraftRow {
    id: String,
    user_id: String,
    channel_id: String,
    content: String,
    attachments: String,
    updated_at: String,
}

fn parse_dt(s: &str) -> Result<DateTime<Utc>, String> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    let normalised = s.replacen(' ', "T", 1);
    let with_colon = if normalised.ends_with("+00") {
        format!("{normalised}:00")
    } else {
        normalised
    };
    chrono::DateTime::parse_from_rfc3339(&with_colon)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("cannot parse timestamp '{s}': {e}"))
}

impl TryFrom<DraftRow> for Draft {
    type Error = NexusError;
    fn try_from(r: DraftRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: r
                .id
                .parse::<Uuid>()
                .map_err(|e| NexusError::Internal(anyhow::anyhow!(e)))?,
            user_id: r
                .user_id
                .parse::<Uuid>()
                .map_err(|e| NexusError::Internal(anyhow::anyhow!(e)))?,
            channel_id: r
                .channel_id
                .parse::<Uuid>()
                .map_err(|e| NexusError::Internal(anyhow::anyhow!(e)))?,
            content: r.content,
            attachments: serde_json::from_str::<Vec<serde_json::Value>>(&r.attachments)
                .unwrap_or_default(),
            updated_at: parse_dt(&r.updated_at)
                .map_err(|e| NexusError::Internal(anyhow::anyhow!(e)))?,
        })
    }
}

// ============================================================
// Handlers
// ============================================================

/// PUT /api/v1/channels/{channel_id}/draft
///
/// Create or replace the caller's draft in this channel.
async fn upsert_draft(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<UpsertDraftRequest>,
) -> NexusResult<Json<Draft>> {
    let draft_id = snowflake::generate_id();
    let attachments_json = serde_json::to_string(&body.attachments.unwrap_or_default())
        .map_err(|e| NexusError::Internal(e.into()))?;

    let row: DraftRow = sqlx::query_as(
        r#"
        INSERT INTO message_drafts (id, user_id, channel_id, content, attachments, updated_at)
        VALUES ($1, $2, $3, $4, $5::jsonb, NOW())
        ON CONFLICT (user_id, channel_id) DO UPDATE
            SET content = EXCLUDED.content,
                attachments = EXCLUDED.attachments,
                updated_at = NOW()
        RETURNING
            id::text, user_id::text, channel_id::text, content,
            attachments::text, updated_at::text
        "#,
    )
    .bind(draft_id.to_string())
    .bind(ctx.user_id.to_string())
    .bind(channel_id.to_string())
    .bind(&body.content)
    .bind(&attachments_json)
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(Draft::try_from(row)?))
}

/// GET /api/v1/channels/{channel_id}/draft
///
/// Retrieve the caller's draft, or 404 if none exists.
async fn get_draft(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Draft>> {
    let row: Option<DraftRow> = sqlx::query_as(
        r#"
        SELECT id::text, user_id::text, channel_id::text, content,
               attachments::text, updated_at::text
        FROM message_drafts
        WHERE user_id = $1 AND channel_id = $2
        "#,
    )
    .bind(ctx.user_id.to_string())
    .bind(channel_id.to_string())
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    match row {
        Some(r) => Ok(Json(Draft::try_from(r)?)),
        None => Err(NexusError::NotFound {
            resource: "Draft".into(),
        }),
    }
}

/// DELETE /api/v1/channels/{channel_id}/draft
///
/// Clear the caller's draft (typically called after successfully sending the message).
async fn delete_draft(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    sqlx::query("DELETE FROM message_drafts WHERE user_id = $1 AND channel_id = $2")
        .bind(ctx.user_id.to_string())
        .bind(channel_id.to_string())
        .execute(&state.db.pool)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    // Idempotent — no error if draft didn't exist
    Ok(Json(serde_json::json!({ "ok": true })))
}
