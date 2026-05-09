//! Canvas (rich document) channel routes — v0.15 Community Ecosystem.
//!
//! Canvas channels are like lightweight wiki pages embedded in a channel.
//! Blocks are ordered by `position` (gaps of 1000 allow easy insertion).
//!
//! GET    /channels/{id}/canvas                       — fetch full block list
//! PUT    /channels/{id}/canvas/blocks/{block_id}     — upsert block
//! DELETE /channels/{id}/canvas/blocks/{block_id}     — remove block
//! POST   /channels/{id}/canvas/blocks/reorder        — update positions

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    middleware,
    routing::{get, post, put},
};
use chrono::{DateTime, Utc};
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::{GatewayEvent, event_types},
    permissions::Permissions,
};
use nexus_db::repository::{members, roles, servers};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{AppState, middleware::AuthContext};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/channels/{channel_id}/canvas", get(get_canvas))
        .route(
            "/channels/{channel_id}/canvas/blocks/{block_id}",
            put(upsert_block).delete(delete_block),
        )
        .route(
            "/channels/{channel_id}/canvas/blocks/reorder",
            post(reorder_blocks),
        )
        .route_layer(middleware::from_fn(
            crate::middleware::combined_auth_middleware,
        ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Public response model
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CanvasBlock {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub block_type: String,
    pub content: serde_json::Value,
    pub position: i32,
    pub updated_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal DB row — all UUID and DateTime columns stored as String for AnyPool
// ─────────────────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct CanvasBlockRow {
    id: String,
    channel_id: String,
    block_type: String,
    content: String, // jsonb cast to ::text
    position: i32,
    updated_by: Option<String>,
    updated_at: String, // timestamptz cast to ::text
}

/// Parse a PostgreSQL timestamptz ::text value into DateTime<Utc>.
fn parse_pg_dt(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
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

impl TryFrom<CanvasBlockRow> for CanvasBlock {
    type Error = NexusError;
    fn try_from(r: CanvasBlockRow) -> Result<Self, Self::Error> {
        Ok(CanvasBlock {
            id: r
                .id
                .parse()
                .map_err(|_| NexusError::Internal(anyhow::anyhow!("invalid block id")))?,
            channel_id: r
                .channel_id
                .parse()
                .map_err(|_| NexusError::Internal(anyhow::anyhow!("invalid channel_id")))?,
            block_type: r.block_type,
            content: serde_json::from_str(&r.content)
                .map_err(|e| NexusError::Internal(anyhow::anyhow!("invalid block content: {e}")))?,
            position: r.position,
            updated_by: r.updated_by.as_deref().and_then(|s| s.parse().ok()),
            updated_at: parse_pg_dt(&r.updated_at).unwrap_or_else(Utc::now),
        })
    }
}

fn rows_to_blocks(rows: Vec<CanvasBlockRow>) -> NexusResult<Vec<CanvasBlock>> {
    rows.into_iter().map(CanvasBlock::try_from).collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Request models
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpsertBlockRequest {
    /// heading | paragraph | image | code | divider | table | callout
    pub block_type: Option<String>,
    pub content: serde_json::Value,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderRequest {
    pub blocks: Vec<BlockPosition>,
}

#[derive(Debug, Deserialize)]
pub struct BlockPosition {
    pub id: Uuid,
    pub position: i32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Permission helpers
// ─────────────────────────────────────────────────────────────────────────────

async fn require_channel_server(state: &AppState, channel_id: Uuid) -> NexusResult<Uuid> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT server_id::text FROM channels WHERE id = $1::uuid")
            .bind(channel_id.to_string())
            .fetch_optional(&state.db.pool)
            .await
            .map_err(NexusError::Database)?;

    row.and_then(|(s,)| s.parse::<Uuid>().ok())
        .ok_or_else(|| NexusError::NotFound {
            resource: "Channel not found or not a server channel".to_string(),
        })
}

async fn require_write_access(
    state: &AppState,
    user_id: Uuid,
    server_id: Uuid,
    need_manage: bool,
) -> NexusResult<()> {
    let server = servers::find_by_id(&state.db.pool, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or_else(|| NexusError::NotFound {
            resource: "Server not found".to_string(),
        })?;

    if server.owner_id == user_id {
        return Ok(());
    }

    // Must be a server member.
    let member = members::find_member(&state.db.pool, user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or(NexusError::Forbidden)?;

    // Compute effective permissions from all server roles.
    let all_roles = roles::list_server_roles(&state.db.pool, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    let base = all_roles
        .iter()
        .find(|r| r.is_default)
        .map(|r| Permissions::from_bits_truncate(r.permissions))
        .unwrap_or_else(Permissions::empty);

    let effective = all_roles
        .iter()
        .filter(|r| !r.is_default && member.roles.contains(&r.id))
        .map(|r| Permissions::from_bits_truncate(r.permissions))
        .fold(base, |acc, p| acc | p);

    if effective.contains(Permissions::ADMINISTRATOR) {
        return Ok(());
    }

    let required = if need_manage {
        Permissions::MANAGE_MESSAGES
    } else {
        Permissions::SEND_MESSAGES
    };

    if !effective.contains(required) {
        return Err(NexusError::Forbidden);
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /channels/{channel_id}/canvas`
async fn get_canvas(
    State(state): State<Arc<AppState>>,
    Extension(_auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Vec<CanvasBlock>>> {
    let rows: Vec<CanvasBlockRow> = sqlx::query_as(
        r#"SELECT id::text AS id, channel_id::text AS channel_id, block_type,
                  content::text AS content, position,
                  updated_by::text AS updated_by, updated_at::text AS updated_at
           FROM canvas_blocks WHERE channel_id = $1::uuid
           ORDER BY position ASC, updated_at ASC"#,
    )
    .bind(channel_id.to_string())
    .fetch_all(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    Ok(Json(rows_to_blocks(rows)?))
}

/// `PUT /channels/{channel_id}/canvas/blocks/{block_id}` — upsert a block
async fn upsert_block(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path((channel_id, block_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpsertBlockRequest>,
) -> NexusResult<Json<CanvasBlock>> {
    let server_id = require_channel_server(&state, channel_id).await?;
    require_write_access(&state, auth.user_id, server_id, false).await?;

    let block_type = body.block_type.unwrap_or_else(|| "paragraph".into());

    let position = if let Some(p) = body.position {
        p
    } else {
        let max_row: Option<(Option<i32>,)> =
            sqlx::query_as("SELECT MAX(position) FROM canvas_blocks WHERE channel_id = $1::uuid")
                .bind(channel_id.to_string())
                .fetch_optional(&state.db.pool)
                .await
                .map_err(NexusError::Database)?;
        max_row.and_then(|(v,)| v).unwrap_or(0) + 1000
    };

    let content_str = body.content.to_string();
    let block: CanvasBlockRow = sqlx::query_as(
        r#"INSERT INTO canvas_blocks (id, channel_id, block_type, content, position, updated_by)
           VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6::uuid)
           ON CONFLICT (id) DO UPDATE
               SET block_type = EXCLUDED.block_type,
                   content    = EXCLUDED.content,
                   position   = EXCLUDED.position,
                   updated_by = EXCLUDED.updated_by,
                   updated_at = NOW()
           RETURNING id::text AS id, channel_id::text AS channel_id,
                     block_type, content::text AS content, position,
                     updated_by::text AS updated_by, updated_at::text AS updated_at"#,
    )
    .bind(block_id.to_string())
    .bind(channel_id.to_string())
    .bind(&block_type)
    .bind(&content_str)
    .bind(position)
    .bind(auth.user_id.to_string())
    .fetch_one(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    let block = CanvasBlock::try_from(block)?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::CANVAS_BLOCK_UPDATE.to_string(),
        data: serde_json::to_value(&block).unwrap_or_default(),
        server_id: Some(server_id),
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(block))
}

/// `DELETE /channels/{channel_id}/canvas/blocks/{block_id}`
async fn delete_block(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path((channel_id, block_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let server_id = require_channel_server(&state, channel_id).await?;
    require_write_access(&state, auth.user_id, server_id, true).await?;

    let result =
        sqlx::query("DELETE FROM canvas_blocks WHERE id = $1::uuid AND channel_id = $2::uuid")
            .bind(block_id.to_string())
            .bind(channel_id.to_string())
            .execute(&state.db.pool)
            .await
            .map_err(NexusError::Database)?;

    if result.rows_affected() == 0 {
        return Err(NexusError::NotFound {
            resource: "Block not found".to_string(),
        });
    }

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::CANVAS_BLOCK_DELETE.to_string(),
        data: serde_json::json!({ "block_id": block_id, "channel_id": channel_id }),
        server_id: Some(server_id),
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// `POST /channels/{channel_id}/canvas/blocks/reorder`
async fn reorder_blocks(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<ReorderRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    let server_id = require_channel_server(&state, channel_id).await?;
    require_write_access(&state, auth.user_id, server_id, false).await?;

    for bp in &body.blocks {
        sqlx::query(
            "UPDATE canvas_blocks SET position = $1, updated_at = NOW() WHERE id = $2::uuid AND channel_id = $3::uuid",
        )
        .bind(bp.position)
        .bind(bp.id.to_string())
        .bind(channel_id.to_string())
        .execute(&state.db.pool)
        .await
        .map_err(NexusError::Database)?;
    }

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::CANVAS_BLOCK_UPDATE.to_string(),
        data: serde_json::json!({ "channel_id": channel_id, "reorder": true }),
        server_id: Some(server_id),
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "reordered": body.blocks.len() })))
}
