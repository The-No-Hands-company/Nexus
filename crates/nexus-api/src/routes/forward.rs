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
use nexus_db::repository::{channels, members};
use serde::{Deserialize, Serialize};
use sqlx::Row;
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
    if body.target_channel_ids.is_empty() {
        return Err(NexusError::Validation {
            message: "target_channel_ids must contain at least one channel".into(),
        });
    }
    if body.target_channel_ids.len() > 10 {
        return Err(NexusError::Validation {
            message: "Cannot forward to more than 10 channels at once".into(),
        });
    }

    // Fetch the original message
    let orig_row = sqlx::query(
        "SELECT channel_id::text AS channel_id, content FROM messages WHERE id = $1::uuid",
    )
    .bind(message_id.to_string())
    .fetch_optional(&state.db.pool)
    .await
    .map_err(NexusError::Database)?
    .ok_or(NexusError::NotFound { resource: "Message".into() })?;

    let original_channel_id: Uuid = orig_row
        .try_get::<String, _>("channel_id")
        .unwrap_or_default()
        .parse()
        .unwrap_or_default();
    let content: String = orig_row
        .try_get::<Option<String>, _>("content")
        .unwrap_or(None)
        .unwrap_or_default();

    let mut created_ids = Vec::new();

    for target_channel_id in &body.target_channel_ids {
        // Verify target channel exists
        let channel = channels::find_by_id(&state.db.pool, *target_channel_id)
            .await?
            .ok_or(NexusError::NotFound { resource: "Target channel".into() })?;

        // Verify the user has SEND_MESSAGES in the target channel
        if let Some(server_id) = channel.server_id {
            let _ = members::find_member(&state.db.pool, auth.user_id, server_id)
                .await?
                .ok_or(NexusError::Forbidden)?;
        }

        let new_id = snowflake::generate_id();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO messages
                (id, channel_id, author_id, content, created_at, updated_at,
                 forwarded_from_message_id, forwarded_from_channel_id)
            VALUES ($1::uuid, $2::uuid, $3::uuid, $4, $5::timestamptz, $5::timestamptz,
                    $6::uuid, $7::uuid)
            "#,
        )
        .bind(new_id.to_string())
        .bind(target_channel_id.to_string())
        .bind(auth.user_id.to_string())
        .bind(&content)
        .bind(now.to_rfc3339())
        .bind(message_id.to_string())
        .bind(original_channel_id.to_string())
        .execute(&state.db.pool)
        .await
        .map_err(NexusError::Database)?;

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
            server_id: channel.server_id,
            channel_id: Some(*target_channel_id),
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
            server_id: channel.server_id,
            channel_id: Some(*target_channel_id),
            user_id: Some(auth.user_id),
        });
    }

    Ok(Json(ForwardResponse {
        forwarded_count: created_ids.len(),
        message_ids: created_ids,
    }))
}
