//! Canvas (rich document) channel routes — v0.15 Community Ecosystem.
//!
//! Canvas channels are like lightweight wiki pages embedded in a channel.
//! Blocks are ordered by `position` (gaps of 1000 allow easy insertion).
//!
//! GET  /channels/{id}/canvas                          — fetch full block list
//! PUT  /channels/{id}/canvas/blocks/{block_id}        — upsert block (SEND_MESSAGES or MANAGE_MESSAGES)
//! DELETE /channels/{id}/canvas/blocks/{block_id}      — remove block (MANAGE_MESSAGES)
//! POST /channels/{id}/canvas/blocks/reorder           — update positions

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{delete, get, post, put},
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
        .route("/channels/{channel_id}/canvas", get(get_canvas))
        .route(
            "/channels/{channel_id}/canvas/blocks/{block_id}",
            put(upsert_block).delete(delete_block),
        )
        .route("/channels/{channel_id}/canvas/blocks/reorder", post(reorder_blocks))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ─────────────────────────────────────────────────────────────────────────────
// Models
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

#[derive(Debug, Deserialize)]
pub struct UpsertBlockRequest {
    /// heading | paragraph | image | code | divider | table | callout
    pub block_type: Option<String>,
    pub content: serde_json::Value,
    /// Position for ordering (gaps of 1000 recommended)
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderRequest {
    /// Ordered list of { id: uuid, position: i32 }
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

async fn require_channel_server(
    state: &AppState,
    channel_id: Uuid,
) -> NexusResult<Uuid> {
    let server_id: Option<String> = sqlx::query_scalar!(
        "SELECT server_id::text FROM channels WHERE id = $1::uuid",
        channel_id.to_string(),
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?
    .flatten();

    server_id
        .and_then(|s| s.parse::<Uuid>().ok())
        .ok_or_else(|| NexusError::NotFound("Channel not found or not a server channel".into()))
}

async fn require_write_access(
    state: &AppState,
    user_id: Uuid,
    server_id: Uuid,
    need_manage: bool,
) -> NexusResult<()> {
    let server = servers::get_server(&state.db.pool, server_id)
        .await
        .map_err(|e| NexusError::Database(e.to_string()))?
        .ok_or_else(|| NexusError::NotFound("Server not found".into()))?;

    if server.owner_id == user_id {
        return Ok(());
    }

    let member_roles = members::get_member_roles(&state.db.pool, server_id, user_id)
        .await
        .map_err(|e| NexusError::Database(e.to_string()))?;
    let perms = roles::calculate_permissions(&state.db.pool, server_id, &member_roles)
        .await
        .map_err(|e| NexusError::Database(e.to_string()))?;
    let bits = Permissions::from_bits_truncate(perms);

    if bits.contains(Permissions::ADMINISTRATOR) {
        return Ok(());
    }

    let required = if need_manage {
        Permissions::MANAGE_MESSAGES
    } else {
        Permissions::SEND_MESSAGES
    };

    if !bits.contains(required) {
        return Err(NexusError::Forbidden("Insufficient permissions for canvas editing".into()));
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /channels/{channel_id}/canvas` — fetch all blocks ordered by position
async fn get_canvas(
    State(state): State<Arc<AppState>>,
    Extension(_auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Vec<CanvasBlock>>> {
    let blocks = sqlx::query_as!(
        CanvasBlock,
        r#"
        SELECT id, channel_id, block_type, content, position, updated_by, updated_at
        FROM canvas_blocks
        WHERE channel_id = $1::uuid
        ORDER BY position ASC, updated_at ASC
        "#,
        channel_id.to_string(),
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

    Ok(Json(blocks))
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

    // Determine position: use provided or default to end (max + 1000)
    let position = if let Some(p) = body.position {
        p
    } else {
        let max: Option<i32> = sqlx::query_scalar!(
            "SELECT MAX(position) FROM canvas_blocks WHERE channel_id = $1::uuid",
            channel_id.to_string(),
        )
        .fetch_optional(&state.db.pool)
        .await
        .map_err(|e| NexusError::Database(e.to_string()))?
        .flatten();
        max.unwrap_or(0) + 1000
    };

    let block = sqlx::query_as!(
        CanvasBlock,
        r#"
        INSERT INTO canvas_blocks (id, channel_id, block_type, content, position, updated_by)
        VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6::uuid)
        ON CONFLICT (id) DO UPDATE
            SET block_type = EXCLUDED.block_type,
                content    = EXCLUDED.content,
                position   = EXCLUDED.position,
                updated_by = EXCLUDED.updated_by,
                updated_at = NOW()
        RETURNING id, channel_id, block_type, content, position, updated_by, updated_at
        "#,
        block_id.to_string(),
        channel_id.to_string(),
        block_type,
        body.content,
        position,
        auth.user_id.to_string(),
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

    // Broadcast live update to all channel subscribers
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::CANVAS_BLOCK_UPDATE.to_string(),
        data: serde_json::to_value(&block).unwrap_or_default(),
        server_id: Some(server_id),
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(block))
}

/// `DELETE /channels/{channel_id}/canvas/blocks/{block_id}` — delete a block (MANAGE_MESSAGES)
async fn delete_block(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path((channel_id, block_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let server_id = require_channel_server(&state, channel_id).await?;
    require_write_access(&state, auth.user_id, server_id, true).await?;

    let deleted = sqlx::query!(
        "DELETE FROM canvas_blocks WHERE id = $1::uuid AND channel_id = $2::uuid",
        block_id.to_string(),
        channel_id.to_string(),
    )
    .execute(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

    if deleted.rows_affected() == 0 {
        return Err(NexusError::NotFound("Block not found".into()));
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

/// `POST /channels/{channel_id}/canvas/blocks/reorder` — bulk reposition blocks
async fn reorder_blocks(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<ReorderRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    let server_id = require_channel_server(&state, channel_id).await?;
    require_write_access(&state, auth.user_id, server_id, false).await?;

    for bp in &body.blocks {
        sqlx::query!(
            "UPDATE canvas_blocks SET position = $1, updated_at = NOW() WHERE id = $2::uuid AND channel_id = $3::uuid",
            bp.position,
            bp.id.to_string(),
            channel_id.to_string(),
        )
        .execute(&state.db.pool)
        .await
        .map_err(|e| NexusError::Database(e.to_string()))?;
    }

    // Broadcast a full resync event
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::CANVAS_BLOCK_UPDATE.to_string(),
        data: serde_json::json!({ "channel_id": channel_id, "reorder": true }),
        server_id: Some(server_id),
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "reordered": body.blocks.len() })))
}
