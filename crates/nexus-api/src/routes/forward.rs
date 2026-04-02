//! Message forwarding routes.
//!
//! Forwards a message to one or more target channels, creating a new message in
//! each that references the original. Attribution is preserved via
//! `forwarded_from_message_id` and `forwarded_from_channel_id` columns.
//!
//! POST /messages/{msg_id}/forward — body: { target_channel_ids: [uuid] }

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::post,
    Json, Router,
};
use chrono::Utc;
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::{event_types, GatewayEvent},
    snowflake,
};
use nexus_db::repository::members;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/messages/{message_id}/forward", post(forward_message))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

#[derive(Debug, Deserialize)]
struct ForwardRequest {
    target_channel_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
struct ForwardResponse {
    forwarded_count: usize,
    message_ids: Vec<Uuid>,
}

/// POST /api/v1/messages/:message_id/forward
async fn forward_message(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(message_id): Path<Uuid>,
    Json(body): Json<ForwardRequest>,
) -> NexusResult<Json<ForwardResponse>> {
    metrics::counter!("nexus_forward_requests_total", "outcome" => "attempt").increment(1);

    let unique_targets: HashSet<Uuid> = body.target_channel_ids.iter().copied().collect();
    if unique_targets.len() != body.target_channel_ids.len() {
        metrics::counter!("nexus_forward_requests_total", "outcome" => "invalid_duplicate_targets").increment(1);
        return Err(NexusError::Validation {
            message: "target_channel_ids must not contain duplicates".into(),
        });
    }

    if body.target_channel_ids.is_empty() {
        metrics::counter!("nexus_forward_requests_total", "outcome" => "invalid_empty_targets").increment(1);
        return Err(NexusError::Validation {
            message: "target_channel_ids must contain at least one channel".into(),
        });
    }
    if body.target_channel_ids.len() > 10 {
        metrics::counter!("nexus_forward_requests_total", "outcome" => "invalid_too_many_targets").increment(1);
        return Err(NexusError::Validation {
            message: "Cannot forward to more than 10 channels at once".into(),
        });
    }

    // Fetch the original message with channel/server context.
    let orig_row = sqlx::query(
        "SELECT m.channel_id::text AS channel_id, m.content AS content, c.server_id::text AS server_id \
         FROM messages m \
         INNER JOIN channels c ON c.id = m.channel_id \
         WHERE m.id = $1::uuid",
    )
    .bind(message_id.to_string())
    .fetch_optional(&state.db.pool)
    .await
    .map_err(NexusError::Database)?
    .ok_or(NexusError::NotFound { resource: "Message".into() })?;

    let original_channel_id: Uuid = orig_row
        .try_get::<String, _>("channel_id")
        .map_err(NexusError::Database)?
        .parse()
        .map_err(|_| NexusError::Validation {
            message: "stored message channel_id is invalid".into(),
        })?;
    let content: String = orig_row
        .try_get::<Option<String>, _>("content")
        .unwrap_or(None)
        .unwrap_or_default();
    let original_server_id = orig_row
        .try_get::<Option<String>, _>("server_id")
        .map_err(NexusError::Database)?
        .map(|s| {
            s.parse::<Uuid>().map_err(|_| NexusError::Validation {
                message: "stored channel server_id is invalid".into(),
            })
        })
        .transpose()?;

    // Authorization guard: user must be able to view the source message channel.
    if let Some(server_id) = original_server_id {
        let is_member = members::is_member(&state.db.pool, auth.user_id, server_id)
            .await
            .map_err(NexusError::Database)?;
        if !is_member {
            metrics::counter!("nexus_forward_requests_total", "outcome" => "forbidden_source_server").increment(1);
            return Err(NexusError::Forbidden);
        }
    } else {
        let source_dm_membership: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM dm_participants WHERE channel_id = $1::uuid AND user_id = $2::uuid)",
        )
        .bind(original_channel_id.to_string())
        .bind(auth.user_id.to_string())
        .fetch_one(&state.db.pool)
        .await
        .map_err(NexusError::Database)?;
        if !source_dm_membership.0 {
            metrics::counter!("nexus_forward_requests_total", "outcome" => "forbidden_source_dm").increment(1);
            return Err(NexusError::Forbidden);
        }
    }

    let mut created_ids = Vec::new();

    // Batch load target channels to avoid per-target lookups.
    let mut qb = sqlx::QueryBuilder::new(
        "SELECT id::text AS id, server_id::text AS server_id FROM channels WHERE id IN (",
    );
    {
        let mut separated = qb.separated(", ");
        for target_channel_id in &body.target_channel_ids {
            separated.push_bind(target_channel_id.to_string());
        }
    }
    qb.push(")");

    let target_rows = qb
        .build()
        .fetch_all(&state.db.pool)
        .await
        .map_err(NexusError::Database)?;

    let mut target_channels: HashMap<Uuid, Option<Uuid>> = HashMap::new();
    for row in target_rows {
        let channel_id = row
            .try_get::<String, _>("id")
            .map_err(NexusError::Database)?
            .parse::<Uuid>()
            .map_err(|_| NexusError::Validation {
                message: "invalid channel id in channels table".into(),
            })?;

        let server_id = row
            .try_get::<Option<String>, _>("server_id")
            .map_err(NexusError::Database)?
            .map(|s| {
                s.parse::<Uuid>().map_err(|_| NexusError::Validation {
                    message: "invalid server id in channels table".into(),
                })
            })
            .transpose()?;

        target_channels.insert(channel_id, server_id);
    }

    if target_channels.len() != body.target_channel_ids.len() {
        metrics::counter!("nexus_forward_requests_total", "outcome" => "not_found_target").increment(1);
        return Err(NexusError::NotFound {
            resource: "One or more target channels not found".into(),
        });
    }

    let target_server_ids: HashSet<Uuid> = target_channels.values().filter_map(|sid| *sid).collect();
    if !target_server_ids.is_empty() {
        let member_server_ids = members::list_server_ids_for_user(&state.db.pool, auth.user_id)
            .await
            .map_err(NexusError::Database)?
            .into_iter()
            .collect::<HashSet<_>>();

        if !target_server_ids.is_subset(&member_server_ids) {
            metrics::counter!("nexus_forward_requests_total", "outcome" => "forbidden_target_server").increment(1);
            return Err(NexusError::Forbidden);
        }
    }

    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let rows_to_insert: Vec<(Uuid, Uuid)> = body
        .target_channel_ids
        .iter()
        .map(|target_channel_id| (snowflake::generate_id(), *target_channel_id))
        .collect();

    let mut qb = sqlx::QueryBuilder::new(
        "INSERT INTO messages (id, channel_id, author_id, content, created_at, updated_at, forwarded_from_message_id, forwarded_from_channel_id) ",
    );
    qb.push_values(rows_to_insert.iter(), |mut b, (new_id, target_channel_id)| {
        b.push_bind(new_id.to_string())
            .push_bind(target_channel_id.to_string())
            .push_bind(auth.user_id.to_string())
            .push_bind(&content)
            .push_bind(&now_str)
            .push_bind(&now_str)
            .push_bind(message_id.to_string())
            .push_bind(original_channel_id.to_string());
    });
    qb.build()
        .execute(&state.db.pool)
        .await
        .map_err(NexusError::Database)?;

    for (new_id, target_channel_id) in rows_to_insert {
        let target_server_id = target_channels.get(&target_channel_id).copied().flatten();
        created_ids.push(new_id);

        // Emit gateway event
        let _ = state.gateway_tx.send(GatewayEvent {
            event_type: event_types::MESSAGE_FORWARD.into(),
            data: serde_json::json!({
                "id": new_id,
                "channel_id": target_channel_id,
                "author_id": auth.user_id,
                "content": content,
                "forwarded_from_message_id": message_id,
                "forwarded_from_channel_id": original_channel_id,
                "created_at": now,
            }),
            server_id: target_server_id,
            channel_id: Some(target_channel_id),
            user_id: Some(auth.user_id),
        });
        // Also emit MESSAGE_CREATE so existing listeners pick it up
        let _ = state.gateway_tx.send(GatewayEvent {
            event_type: event_types::MESSAGE_CREATE.into(),
            data: serde_json::json!({
                "id": new_id,
                "channel_id": target_channel_id,
                "author_id": auth.user_id,
                "content": content,
                "forwarded_from_message_id": message_id,
                "forwarded_from_channel_id": original_channel_id,
                "created_at": now,
            }),
            server_id: target_server_id,
            channel_id: Some(target_channel_id),
            user_id: Some(auth.user_id),
        });
    }

    metrics::counter!("nexus_forward_requests_total", "outcome" => "ok").increment(1);
    metrics::counter!("nexus_forward_messages_total").increment(created_ids.len() as u64);

    Ok(Json(ForwardResponse {
        forwarded_count: created_ids.len(),
        message_ids: created_ids,
    }))
}
