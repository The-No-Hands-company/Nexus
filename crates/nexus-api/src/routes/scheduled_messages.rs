//! Scheduled message routes — create, list, edit, cancel.
//!
//! Scheduled messages are sent automatically by a background task at
//! the specified future time.  The background task calls into the
//! normal messages pipeline so the resulting message is identical to
//! one sent interactively.
//!
//! POST   /channels/:id/scheduled-messages                — Create
//! GET    /channels/:id/scheduled-messages                — List pending
//! PATCH  /channels/:id/scheduled-messages/:sm_id         — Edit / reschedule
//! DELETE /channels/:id/scheduled-messages/:sm_id         — Cancel

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use nexus_common::{
    error::{NexusError, NexusResult},
    snowflake,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// ============================================================
// Router
// ============================================================

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/channels/{channel_id}/scheduled-messages",
            get(list_scheduled).post(create_scheduled),
        )
        .route(
            "/channels/{channel_id}/scheduled-messages/{sm_id}",
            axum::routing::patch(update_scheduled).delete(cancel_scheduled),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ============================================================
// Models
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledMessage {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub attachments: Vec<serde_json::Value>,
    pub scheduled_at: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateScheduledRequest {
    content: String,
    scheduled_at: DateTime<Utc>,
    attachments: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct UpdateScheduledRequest {
    content: Option<String>,
    scheduled_at: Option<DateTime<Utc>>,
    attachments: Option<Vec<serde_json::Value>>,
}

// ============================================================
// DB row types
// ============================================================

#[derive(sqlx::FromRow)]
struct ScheduledRow {
    id: String,
    channel_id: String,
    author_id: String,
    content: String,
    attachments: String,  // jsonb as text
    scheduled_at: String,
    status: String,
    created_at: String,
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

impl TryFrom<ScheduledRow> for ScheduledMessage {
    type Error = NexusError;

    fn try_from(r: ScheduledRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: r.id.parse().map_err(|e| NexusError::Internal(anyhow::anyhow!(e)))?,
            channel_id: r.channel_id.parse().map_err(|e| NexusError::Internal(anyhow::anyhow!(e)))?,
            author_id: r.author_id.parse().map_err(|e| NexusError::Internal(anyhow::anyhow!(e)))?,
            content: r.content,
            attachments: serde_json::from_str::<Vec<serde_json::Value>>(&r.attachments)
                .unwrap_or_default(),
            scheduled_at: parse_dt(&r.scheduled_at)
                .map_err(|e| NexusError::Internal(anyhow::anyhow!(e)))?,
            status: r.status,
            created_at: parse_dt(&r.created_at)
                .map_err(|e| NexusError::Internal(anyhow::anyhow!(e)))?,
            updated_at: parse_dt(&r.updated_at)
                .map_err(|e| NexusError::Internal(anyhow::anyhow!(e)))?,
        })
    }
}

// ============================================================
// Handlers
// ============================================================

/// POST /api/v1/channels/{channel_id}/scheduled-messages
async fn create_scheduled(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateScheduledRequest>,
) -> NexusResult<Json<ScheduledMessage>> {
    if body.scheduled_at <= Utc::now() {
        return Err(NexusError::Validation {
            message: "scheduled_at must be in the future".into(),
        });
    }
    if body.content.trim().is_empty() {
        return Err(NexusError::Validation { message: "Message content cannot be empty".into() });
    }

    let sm_id = snowflake::generate_id();
    let attachments_json = serde_json::to_string(
        &body.attachments.unwrap_or_default(),
    )
    .map_err(|e| NexusError::Internal(e.into()))?;

    let row: ScheduledRow = sqlx::query_as(
        r#"
        INSERT INTO scheduled_messages
            (id, channel_id, author_id, content, attachments, scheduled_at,
             status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5::jsonb, $6, 'pending', NOW(), NOW())
        RETURNING
            id::text, channel_id::text, author_id::text, content,
            attachments::text, scheduled_at::text, status,
            created_at::text, updated_at::text
        "#,
    )
    .bind(sm_id.to_string())
    .bind(channel_id.to_string())
    .bind(ctx.user_id.to_string())
    .bind(&body.content)
    .bind(&attachments_json)
    .bind(body.scheduled_at.to_rfc3339())
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(ScheduledMessage::try_from(row)?))
}

/// GET /api/v1/channels/{channel_id}/scheduled-messages
async fn list_scheduled(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Vec<ScheduledMessage>>> {
    // Users can see their own scheduled messages only
    let rows: Vec<ScheduledRow> = sqlx::query_as(
        r#"
        SELECT id::text, channel_id::text, author_id::text, content,
               attachments::text, scheduled_at::text, status,
               created_at::text, updated_at::text
        FROM scheduled_messages
        WHERE channel_id = $1 AND author_id = $2 AND status = 'pending'
        ORDER BY scheduled_at ASC
        "#,
    )
    .bind(channel_id.to_string())
    .bind(ctx.user_id.to_string())
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    let messages: Result<Vec<_>, _> = rows.into_iter().map(ScheduledMessage::try_from).collect();
    Ok(Json(messages?))
}

/// PATCH /api/v1/channels/{channel_id}/scheduled-messages/{sm_id}
async fn update_scheduled(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path((_channel_id, sm_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateScheduledRequest>,
) -> NexusResult<Json<ScheduledMessage>> {
    if let Some(scheduled_at) = body.scheduled_at {
        if scheduled_at <= Utc::now() {
            return Err(NexusError::Validation {
                message: "scheduled_at must be in the future".into(),
            });
        }
    }

    // Fetch existing — must be owned and pending
    let existing: Option<ScheduledRow> = sqlx::query_as(
        r#"
        SELECT id::text, channel_id::text, author_id::text, content,
               attachments::text, scheduled_at::text, status,
               created_at::text, updated_at::text
        FROM scheduled_messages
        WHERE id = $1 AND author_id = $2 AND status = 'pending'
        "#,
    )
    .bind(sm_id.to_string())
    .bind(ctx.user_id.to_string())
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    let current = match existing {
        Some(r) => ScheduledMessage::try_from(r)?,
        None => return Err(NexusError::NotFound { resource: "Scheduled message".into() }),
    };

    let new_content = body.content.as_deref().unwrap_or(&current.content);
    let new_scheduled_at = body.scheduled_at.unwrap_or(current.scheduled_at);
    let new_attachments_json = match body.attachments {
        Some(ref a) => serde_json::to_string(a).map_err(|e| NexusError::Internal(e.into()))?,
        None => serde_json::to_string(&current.attachments)
            .map_err(|e| NexusError::Internal(e.into()))?,
    };

    let row: ScheduledRow = sqlx::query_as(
        r#"
        UPDATE scheduled_messages
        SET content = $1, scheduled_at = $2, attachments = $3::jsonb, updated_at = NOW()
        WHERE id = $4
        RETURNING
            id::text, channel_id::text, author_id::text, content,
            attachments::text, scheduled_at::text, status,
            created_at::text, updated_at::text
        "#,
    )
    .bind(new_content)
    .bind(new_scheduled_at.to_rfc3339())
    .bind(&new_attachments_json)
    .bind(sm_id.to_string())
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(ScheduledMessage::try_from(row)?))
}

/// DELETE /api/v1/channels/{channel_id}/scheduled-messages/{sm_id}
async fn cancel_scheduled(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path((_channel_id, sm_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let result = sqlx::query(
        r#"
        UPDATE scheduled_messages
        SET status = 'cancelled', updated_at = NOW()
        WHERE id = $1 AND author_id = $2 AND status = 'pending'
        "#,
    )
    .bind(sm_id.to_string())
    .bind(ctx.user_id.to_string())
    .execute(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    if result.rows_affected() == 0 {
        return Err(NexusError::NotFound { resource: "Scheduled message".into() });
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}
