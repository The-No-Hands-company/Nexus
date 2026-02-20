//! DM (Direct Message) routes — create and list DM channels.
//!
//! DMs are just channels with type "dm" or "group_dm" and dm_participants entries.
//! Messages in DMs use the same /channels/:id/messages endpoints.

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::get,
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::channel::Channel,
    snowflake,
};
use nexus_db::repository::channels;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

/// DM routes — mounted under /users/@me/channels.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/users/@me/channels",
            get(list_dm_channels).post(create_dm),
        )
        .route(
            "/users/@me/channels/{channel_id}",
            get(get_dm_channel),
        )
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

#[derive(Debug, Deserialize)]
struct CreateDmRequest {
    /// User ID to open a DM with (1:1 DM)
    recipient_id: Option<Uuid>,
    /// Multiple user IDs for group DM
    recipient_ids: Option<Vec<Uuid>>,
    /// Group DM name (optional, only for group DMs)
    name: Option<String>,
}

/// GET /api/v1/users/@me/channels — List all DM channels for the current user.
async fn list_dm_channels(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<Vec<serde_json::Value>>> {
    let dms = sqlx::query_as::<_, Channel>(
        r#"
        SELECT c.* FROM channels c
        INNER JOIN dm_participants dp ON dp.channel_id = c.id
        WHERE dp.user_id = ? AND c.channel_type IN ('dm', 'group_dm')
        ORDER BY c.updated_at DESC
        "#,
    )
    .bind(auth.user_id.to_string())
    .fetch_all(&state.db.pool)
    .await?;

    // For each DM, fetch the other participants
    let mut results = Vec::with_capacity(dms.len());
    for dm in &dms {
        let participants: Vec<(String,)> = sqlx::query_as(
            "SELECT user_id FROM dm_participants WHERE channel_id = ?",
        )
        .bind(dm.id.to_string())
        .fetch_all(&state.db.pool)
        .await?;

        let participant_ids: Vec<Uuid> = participants.into_iter().filter_map(|p| p.0.parse().ok()).collect();

        // Fetch participant user info
        let mut users = Vec::new();
        for &uid in &participant_ids {
            if let Some(user) = nexus_db::repository::users::find_by_id(&state.db.pool, uid).await? {
                users.push(serde_json::json!({
                    "id": user.id,
                    "username": user.username,
                    "display_name": user.display_name,
                    "avatar": user.avatar,
                }));
            }
        }

        results.push(serde_json::json!({
            "id": dm.id,
            "channel_type": dm.channel_type,
            "name": dm.name,
            "last_message_id": dm.last_message_id,
            "recipients": users,
            "updated_at": dm.updated_at,
        }));
    }

    Ok(Json(results))
}

/// POST /api/v1/users/@me/channels — Create a DM channel (or return existing).
async fn create_dm(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateDmRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    if let Some(recipient_id) = body.recipient_id {
        // 1:1 DM — find or create
        if recipient_id == auth.user_id {
            return Err(NexusError::Validation {
                message: "Cannot DM yourself".into(),
            });
        }

        // Verify recipient exists
        nexus_db::repository::users::find_by_id(&state.db.pool, recipient_id)
            .await?
            .ok_or(NexusError::NotFound {
                resource: "User".into(),
            })?;

        let dm_id = snowflake::generate_id();
        let dm = channels::find_or_create_dm(&state.db.pool, dm_id, auth.user_id, recipient_id)
            .await?;

        // Fetch recipient info
        let recipient = nexus_db::repository::users::find_by_id(&state.db.pool, recipient_id)
            .await?
            .ok_or(NexusError::NotFound { resource: "User".into() })?;

        Ok(Json(serde_json::json!({
            "id": dm.id,
            "channel_type": "dm",
            "recipients": [{
                "id": recipient.id,
                "username": recipient.username,
                "display_name": recipient.display_name,
                "avatar": recipient.avatar,
            }],
            "last_message_id": dm.last_message_id,
        })))
    } else if let Some(recipient_ids) = body.recipient_ids {
        // Group DM
        if recipient_ids.len() < 2 {
            return Err(NexusError::Validation {
                message: "Group DM requires at least 2 other users".into(),
            });
        }
        if recipient_ids.len() > 9 {
            return Err(NexusError::Validation {
                message: "Group DM can have at most 10 participants".into(),
            });
        }
        if recipient_ids.contains(&auth.user_id) {
            return Err(NexusError::Validation {
                message: "Do not include yourself in recipient_ids".into(),
            });
        }

        let channel_id = snowflake::generate_id();
        let name = body.name.as_deref();

        // Create group DM channel
        let channel = channels::create_channel(
            &state.db.pool,
            channel_id,
            None,
            None,
            "group_dm",
            name,
            None,
            0,
        )
        .await?;

        // Add all participants (including creator)
        let mut all_participants = recipient_ids.clone();
        all_participants.push(auth.user_id);

        for &uid in &all_participants {
            sqlx::query("INSERT INTO dm_participants (channel_id, user_id) VALUES (?, ?) ON CONFLICT DO NOTHING")
                .bind(channel_id.to_string())
                .bind(uid.to_string())
                .execute(&state.db.pool)
                .await?;
        }

        // Fetch participant info
        let mut users = Vec::new();
        for &uid in &all_participants {
            if let Some(user) = nexus_db::repository::users::find_by_id(&state.db.pool, uid).await? {
                users.push(serde_json::json!({
                    "id": user.id,
                    "username": user.username,
                    "display_name": user.display_name,
                    "avatar": user.avatar,
                }));
            }
        }

        Ok(Json(serde_json::json!({
            "id": channel.id,
            "channel_type": "group_dm",
            "name": channel.name,
            "recipients": users,
            "last_message_id": channel.last_message_id,
        })))
    } else {
        Err(NexusError::Validation {
            message: "Must provide recipient_id (1:1 DM) or recipient_ids (group DM)".into(),
        })
    }
}

/// GET /api/v1/users/@me/channels/:channel_id — Get a specific DM channel.
async fn get_dm_channel(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    // Verify user is a participant
    let is_participant: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM dm_participants WHERE channel_id = ? AND user_id = ?)",
    )
    .bind(channel_id.to_string())
    .bind(auth.user_id.to_string())
    .fetch_one(&state.db.pool)
    .await?;

    if !is_participant.0 {
        return Err(NexusError::NotFound {
            resource: "Channel".into(),
        });
    }

    let channel = channels::find_by_id(&state.db.pool, channel_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

    let participants: Vec<(String,)> = sqlx::query_as(
        "SELECT user_id FROM dm_participants WHERE channel_id = ?",
    )
    .bind(channel_id.to_string())
    .fetch_all(&state.db.pool)
    .await?;

    let mut users = Vec::new();
    for (uid_str,) in &participants {
        let uid: Uuid = match uid_str.parse() {
            Ok(u) => u,
            Err(_) => continue,
        };
        if let Some(user) = nexus_db::repository::users::find_by_id(&state.db.pool, uid).await? {
            users.push(serde_json::json!({
                "id": user.id,
                "username": user.username,
                "display_name": user.display_name,
                "avatar": user.avatar,
            }));
        }
    }

    Ok(Json(serde_json::json!({
        "id": channel.id,
        "channel_type": channel.channel_type,
        "name": channel.name,
        "recipients": users,
        "last_message_id": channel.last_message_id,
        "updated_at": channel.updated_at,
    })))
}
