//! Message routes — send, edit, delete, history, search, pins, reactions.
//!
//! This is the core of chat. Every message mutation emits a gateway event
//! so connected WebSocket clients see changes in real-time.

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::message::{CreateMessageRequest, UpdateMessageRequest},
    snowflake,
    validation::validate_request,
};
use nexus_db::repository::{channels, members, messages, reactions, read_states};
use nexus_common::gateway_event::GatewayEvent;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

/// Message routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Message CRUD
        .route(
            "/channels/{channel_id}/messages",
            get(get_messages).post(send_message),
        )
        .route(
            "/channels/{channel_id}/messages/{message_id}",
            get(get_message)
                .patch(edit_message)
                .delete(delete_message),
        )
        // Bulk delete
        .route(
            "/channels/{channel_id}/messages/bulk-delete",
            post(bulk_delete_messages),
        )
        // Pins
        .route(
            "/channels/{channel_id}/pins",
            get(get_pinned_messages),
        )
        .route(
            "/channels/{channel_id}/pins/{message_id}",
            put(pin_message).delete(unpin_message),
        )
        // Reactions
        .route(
            "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me",
            put(add_reaction).delete(remove_reaction),
        )
        .route(
            "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}",
            get(get_reactors).delete(remove_all_emoji_reactions),
        )
        .route(
            "/channels/{channel_id}/messages/{message_id}/reactions",
            delete(remove_all_reactions),
        )
        // Read state
        .route(
            "/channels/{channel_id}/ack/{message_id}",
            post(ack_message),
        )
        // Search
        .route("/channels/{channel_id}/search", get(search_messages))
        // All routes require authentication
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

// ============================================================================
// Query parameters
// ============================================================================

#[derive(Debug, Deserialize)]
struct MessageHistoryParams {
    before: Option<Uuid>,
    after: Option<Uuid>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    query: String,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct BulkDeleteBody {
    messages: Vec<Uuid>,
}

// ============================================================================
// Message CRUD
// ============================================================================

/// POST /api/v1/channels/:channel_id/messages — Send a message.
async fn send_message(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateMessageRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    validate_request(&body)?;

    // Verify channel exists
    let channel = channels::find_by_id(&state.db.pg, channel_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Channel".into(),
        })?;

    // If this is a server channel, verify user is a member
    if let Some(server_id) = channel.server_id {
        if !members::is_member(&state.db.pg, auth.user_id, server_id).await? {
            return Err(NexusError::Forbidden);
        }
    }

    // Determine message type: 0 = Default, 1 = Reply (if reference provided)
    let message_type = if body.reference.is_some() { 1 } else { 0 };

    let (ref_msg_id, ref_ch_id) = match &body.reference {
        Some(r) => (Some(r.message_id), Some(r.channel_id)),
        None => (None, None),
    };

    // Parse mentions from content (basic @user_id pattern)
    let mentions = parse_mentions(&body.content);
    let mention_everyone = body.content.contains("@everyone");

    let message_id = snowflake::generate_id();
    let msg = messages::create_message(
        &state.db.pg,
        message_id,
        channel_id,
        auth.user_id,
        &body.content,
        message_type,
        ref_msg_id,
        ref_ch_id,
        &mentions,
        &[],
        mention_everyone,
    )
    .await?;

    // Increment mention counts for mentioned users
    for mentioned_user_id in &mentions {
        let _ = read_states::increment_mention_count(
            &state.db.pg,
            *mentioned_user_id,
            channel_id,
        )
        .await;
    }

    let response = message_row_to_json(&msg, &[]);

    // Emit MESSAGE_CREATE event to gateway
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "MESSAGE_CREATE".into(),
        data: response.clone(),
        server_id: channel.server_id,
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    tracing::debug!(
        message_id = %message_id,
        channel_id = %channel_id,
        author = %auth.username,
        "Message sent"
    );

    Ok(Json(response))
}

/// GET /api/v1/channels/:channel_id/messages — Get message history.
async fn get_messages(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Query(params): Query<MessageHistoryParams>,
) -> NexusResult<Json<Vec<serde_json::Value>>> {
    // Verify channel exists
    let channel = channels::find_by_id(&state.db.pg, channel_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Channel".into(),
        })?;

    // If server channel, verify membership
    if let Some(server_id) = channel.server_id {
        if !members::is_member(&state.db.pg, auth.user_id, server_id).await? {
            return Err(NexusError::Forbidden);
        }
    }

    let limit = params.limit.unwrap_or(50).min(100).max(1);
    let rows = messages::list_channel_messages(
        &state.db.pg,
        channel_id,
        params.before,
        params.after,
        limit,
    )
    .await?;

    // Fetch reactions for all messages in batch
    let mut result = Vec::with_capacity(rows.len());
    for row in &rows {
        let reaction_counts = reactions::get_reaction_counts(&state.db.pg, row.id)
            .await
            .unwrap_or_default();
        let my_reactions = get_user_reactions(&state, row.id, auth.user_id, &reaction_counts).await;
        result.push(message_row_to_json_with_reactions(row, &reaction_counts, &my_reactions));
    }

    Ok(Json(result))
}

/// GET /api/v1/channels/:channel_id/messages/:message_id — Get a single message.
async fn get_message(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let msg = messages::find_by_id(&state.db.pg, message_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Message".into(),
        })?;

    if msg.channel_id != channel_id {
        return Err(NexusError::NotFound {
            resource: "Message".into(),
        });
    }

    let reaction_counts = reactions::get_reaction_counts(&state.db.pg, message_id)
        .await
        .unwrap_or_default();

    Ok(Json(message_row_to_json(&msg, &reaction_counts)))
}

/// PATCH /api/v1/channels/:channel_id/messages/:message_id — Edit a message.
async fn edit_message(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateMessageRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    validate_request(&body)?;

    let msg = messages::find_by_id(&state.db.pg, message_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Message".into(),
        })?;

    // Only the author can edit their own message
    if msg.author_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }

    if msg.channel_id != channel_id {
        return Err(NexusError::NotFound {
            resource: "Message".into(),
        });
    }

    let content = body.content.as_deref().ok_or(NexusError::Validation {
        message: "Content is required".into(),
    })?;

    let updated = messages::update_message(&state.db.pg, message_id, content).await?;

    let channel = channels::find_by_id(&state.db.pg, channel_id).await?.ok_or(NexusError::NotFound {
        resource: "Channel".into(),
    })?;

    let response = message_row_to_json(&updated, &[]);

    // Emit MESSAGE_UPDATE event
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "MESSAGE_UPDATE".into(),
        data: response.clone(),
        server_id: channel.server_id,
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(response))
}

/// DELETE /api/v1/channels/:channel_id/messages/:message_id — Delete a message.
async fn delete_message(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let msg = messages::find_by_id(&state.db.pg, message_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Message".into(),
        })?;

    if msg.channel_id != channel_id {
        return Err(NexusError::NotFound {
            resource: "Message".into(),
        });
    }

    // Author can delete their own, or MANAGE_MESSAGES permission in server channels
    let channel = channels::find_by_id(&state.db.pg, channel_id).await?.ok_or(NexusError::NotFound {
        resource: "Channel".into(),
    })?;

    if msg.author_id != auth.user_id {
        // Check if user has MANAGE_MESSAGES permission
        if let Some(server_id) = channel.server_id {
            let server = nexus_db::repository::servers::find_by_id(&state.db.pg, server_id)
                .await?
                .ok_or(NexusError::NotFound { resource: "Server".into() })?;
            if server.owner_id != auth.user_id {
                return Err(NexusError::MissingPermission {
                    permission: "MANAGE_MESSAGES".into(),
                });
            }
        } else {
            return Err(NexusError::Forbidden);
        }
    }

    messages::delete_message(&state.db.pg, message_id).await?;

    // Emit MESSAGE_DELETE event
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "MESSAGE_DELETE".into(),
        data: serde_json::json!({
            "id": message_id,
            "channel_id": channel_id,
            "server_id": channel.server_id,
        }),
        server_id: channel.server_id,
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// POST /api/v1/channels/:channel_id/messages/bulk-delete
async fn bulk_delete_messages(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<BulkDeleteBody>,
) -> NexusResult<Json<serde_json::Value>> {
    if body.messages.is_empty() || body.messages.len() > 100 {
        return Err(NexusError::Validation {
            message: "Must delete between 1 and 100 messages".into(),
        });
    }

    let channel = channels::find_by_id(&state.db.pg, channel_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Channel".into(),
        })?;

    // Must be server owner or have MANAGE_MESSAGES
    if let Some(server_id) = channel.server_id {
        let server = nexus_db::repository::servers::find_by_id(&state.db.pg, server_id)
            .await?
            .ok_or(NexusError::NotFound { resource: "Server".into() })?;
        if server.owner_id != auth.user_id {
            return Err(NexusError::MissingPermission {
                permission: "MANAGE_MESSAGES".into(),
            });
        }
    } else {
        return Err(NexusError::Forbidden);
    }

    let deleted = messages::bulk_delete_messages(&state.db.pg, &body.messages).await?;

    // Emit MESSAGE_BULK_DELETE event
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "MESSAGE_BULK_DELETE".into(),
        data: serde_json::json!({
            "ids": body.messages,
            "channel_id": channel_id,
            "server_id": channel.server_id,
        }),
        server_id: channel.server_id,
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

// ============================================================================
// Pins
// ============================================================================

/// GET /api/v1/channels/:channel_id/pins
async fn get_pinned_messages(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Vec<serde_json::Value>>> {
    let rows = messages::get_pinned_messages(&state.db.pg, channel_id).await?;
    let result: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| message_row_to_json(r, &[]))
        .collect();
    Ok(Json(result))
}

/// PUT /api/v1/channels/:channel_id/pins/:message_id
async fn pin_message(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let msg = messages::find_by_id(&state.db.pg, message_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Message".into() })?;

    if msg.channel_id != channel_id {
        return Err(NexusError::NotFound { resource: "Message".into() });
    }

    let channel = channels::find_by_id(&state.db.pg, channel_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

    // Check permission — for now, any member can pin in DMs, owner in servers
    if let Some(server_id) = channel.server_id {
        let server = nexus_db::repository::servers::find_by_id(&state.db.pg, server_id)
            .await?
            .ok_or(NexusError::NotFound { resource: "Server".into() })?;
        if server.owner_id != auth.user_id {
            return Err(NexusError::MissingPermission {
                permission: "MANAGE_MESSAGES".into(),
            });
        }
    }

    let pinned = messages::pin_message(&state.db.pg, message_id).await?;
    let response = message_row_to_json(&pinned, &[]);

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "CHANNEL_PINS_UPDATE".into(),
        data: serde_json::json!({
            "channel_id": channel_id,
            "message": response,
        }),
        server_id: channel.server_id,
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "pinned": true })))
}

/// DELETE /api/v1/channels/:channel_id/pins/:message_id
async fn unpin_message(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let msg = messages::find_by_id(&state.db.pg, message_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Message".into() })?;

    if msg.channel_id != channel_id {
        return Err(NexusError::NotFound { resource: "Message".into() });
    }

    messages::unpin_message(&state.db.pg, message_id).await?;

    let channel = channels::find_by_id(&state.db.pg, channel_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "CHANNEL_PINS_UPDATE".into(),
        data: serde_json::json!({
            "channel_id": channel_id,
            "unpinned_message_id": message_id,
        }),
        server_id: channel.server_id,
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "unpinned": true })))
}

// ============================================================================
// Reactions
// ============================================================================

/// PUT /api/v1/channels/:channel_id/messages/:message_id/reactions/:emoji/@me
async fn add_reaction(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, message_id, emoji)): Path<(Uuid, Uuid, String)>,
) -> NexusResult<Json<serde_json::Value>> {
    // Verify message exists in channel
    let msg = messages::find_by_id(&state.db.pg, message_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Message".into() })?;

    if msg.channel_id != channel_id {
        return Err(NexusError::NotFound { resource: "Message".into() });
    }

    let added = reactions::add_reaction(&state.db.pg, message_id, auth.user_id, &emoji).await?;

    if added {
        let channel = channels::find_by_id(&state.db.pg, channel_id)
            .await?
            .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

        let _ = state.gateway_tx.send(GatewayEvent {
            event_type: "MESSAGE_REACTION_ADD".into(),
            data: serde_json::json!({
                "message_id": message_id,
                "channel_id": channel_id,
                "user_id": auth.user_id,
                "emoji": emoji,
            }),
            server_id: channel.server_id,
            channel_id: Some(channel_id),
            user_id: Some(auth.user_id),
        });
    }

    Ok(Json(serde_json::json!({ "added": added })))
}

/// DELETE /api/v1/channels/:channel_id/messages/:message_id/reactions/:emoji/@me
async fn remove_reaction(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, message_id, emoji)): Path<(Uuid, Uuid, String)>,
) -> NexusResult<Json<serde_json::Value>> {
    let removed = reactions::remove_reaction(&state.db.pg, message_id, auth.user_id, &emoji).await?;

    if removed {
        let channel = channels::find_by_id(&state.db.pg, channel_id)
            .await?
            .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

        let _ = state.gateway_tx.send(GatewayEvent {
            event_type: "MESSAGE_REACTION_REMOVE".into(),
            data: serde_json::json!({
                "message_id": message_id,
                "channel_id": channel_id,
                "user_id": auth.user_id,
                "emoji": emoji,
            }),
            server_id: channel.server_id,
            channel_id: Some(channel_id),
            user_id: Some(auth.user_id),
        });
    }

    Ok(Json(serde_json::json!({ "removed": removed })))
}

/// GET /api/v1/channels/:channel_id/messages/:message_id/reactions/:emoji
async fn get_reactors(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((_channel_id, message_id, emoji)): Path<(Uuid, Uuid, String)>,
) -> NexusResult<Json<Vec<Uuid>>> {
    let users = reactions::get_reactors(&state.db.pg, message_id, &emoji, 100).await?;
    Ok(Json(users))
}

/// DELETE /api/v1/channels/:channel_id/messages/:message_id/reactions/:emoji — Remove all of one emoji
async fn remove_all_emoji_reactions(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, message_id, emoji)): Path<(Uuid, Uuid, String)>,
) -> NexusResult<Json<serde_json::Value>> {
    // Only server owner / MANAGE_MESSAGES can bulk-remove reactions
    let channel = channels::find_by_id(&state.db.pg, channel_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

    if let Some(server_id) = channel.server_id {
        let server = nexus_db::repository::servers::find_by_id(&state.db.pg, server_id)
            .await?
            .ok_or(NexusError::NotFound { resource: "Server".into() })?;
        if server.owner_id != auth.user_id {
            return Err(NexusError::MissingPermission {
                permission: "MANAGE_MESSAGES".into(),
            });
        }
    }

    let count = reactions::remove_all_reactions_for_emoji(&state.db.pg, message_id, &emoji).await?;
    Ok(Json(serde_json::json!({ "removed": count })))
}

/// DELETE /api/v1/channels/:channel_id/messages/:message_id/reactions — Remove all reactions
async fn remove_all_reactions(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let channel = channels::find_by_id(&state.db.pg, channel_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

    if let Some(server_id) = channel.server_id {
        let server = nexus_db::repository::servers::find_by_id(&state.db.pg, server_id)
            .await?
            .ok_or(NexusError::NotFound { resource: "Server".into() })?;
        if server.owner_id != auth.user_id {
            return Err(NexusError::MissingPermission {
                permission: "MANAGE_MESSAGES".into(),
            });
        }
    }

    let count = reactions::remove_all_reactions(&state.db.pg, message_id).await?;
    Ok(Json(serde_json::json!({ "removed": count })))
}

// ============================================================================
// Read state
// ============================================================================

/// POST /api/v1/channels/:channel_id/ack/:message_id — Acknowledge reading up to a message.
async fn ack_message(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let rs = read_states::ack_message(&state.db.pg, auth.user_id, channel_id, message_id).await?;

    Ok(Json(serde_json::json!({
        "channel_id": rs.channel_id,
        "last_read_message_id": rs.last_read_message_id,
        "mention_count": rs.mention_count,
    })))
}

// ============================================================================
// Search
// ============================================================================

/// GET /api/v1/channels/:channel_id/search?query=...&limit=...&offset=...
async fn search_messages(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Query(params): Query<SearchParams>,
) -> NexusResult<Json<Vec<serde_json::Value>>> {
    // Verify access
    let channel = channels::find_by_id(&state.db.pg, channel_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

    if let Some(server_id) = channel.server_id {
        if !members::is_member(&state.db.pg, auth.user_id, server_id).await? {
            return Err(NexusError::Forbidden);
        }
    }

    let limit = params.limit.unwrap_or(25);
    let offset = params.offset.unwrap_or(0);

    let rows = messages::search_messages(
        &state.db.pg,
        Some(channel_id),
        &params.query,
        limit,
        offset,
    )
    .await?;

    let result: Vec<serde_json::Value> = rows.iter().map(|r| message_row_to_json(r, &[])).collect();
    Ok(Json(result))
}

// ============================================================================
// Helpers
// ============================================================================

/// Convert a MessageRow to a JSON response.
fn message_row_to_json(
    row: &messages::MessageRow,
    reaction_counts: &[reactions::ReactionCount],
) -> serde_json::Value {
    message_row_to_json_with_reactions(row, reaction_counts, &[])
}

fn message_row_to_json_with_reactions(
    row: &messages::MessageRow,
    reaction_counts: &[reactions::ReactionCount],
    my_reactions: &[String],
) -> serde_json::Value {
    let reactions_json: Vec<serde_json::Value> = reaction_counts
        .iter()
        .map(|rc| {
            serde_json::json!({
                "emoji": rc.emoji,
                "count": rc.count,
                "me": my_reactions.contains(&rc.emoji),
            })
        })
        .collect();

    let reference = match (row.reference_message_id, row.reference_channel_id) {
        (Some(mid), Some(cid)) => Some(serde_json::json!({
            "message_id": mid,
            "channel_id": cid,
        })),
        _ => None,
    };

    serde_json::json!({
        "id": row.id,
        "channel_id": row.channel_id,
        "author_id": row.author_id,
        "content": row.content,
        "message_type": row.message_type,
        "edited": row.edited,
        "edited_at": row.edited_at,
        "pinned": row.pinned,
        "embeds": row.embeds,
        "attachments": row.attachments,
        "mentions": row.mentions,
        "mention_roles": row.mention_roles,
        "mention_everyone": row.mention_everyone,
        "reference": reference,
        "thread_id": row.thread_id,
        "reactions": reactions_json,
        "created_at": row.created_at,
    })
}

/// Parse @<uuid> mentions from message content.
fn parse_mentions(content: &str) -> Vec<Uuid> {
    let mut mentions = Vec::new();
    for part in content.split_whitespace() {
        if let Some(id_str) = part.strip_prefix("<@").and_then(|s| s.strip_suffix('>')) {
            if let Ok(id) = id_str.parse::<Uuid>() {
                if !mentions.contains(&id) {
                    mentions.push(id);
                }
            }
        }
    }
    mentions
}

/// Get which emojis the current user has reacted with on a message.
async fn get_user_reactions(
    state: &AppState,
    message_id: Uuid,
    user_id: Uuid,
    reaction_counts: &[reactions::ReactionCount],
) -> Vec<String> {
    let mut my_reactions = Vec::new();
    for rc in reaction_counts {
        if reactions::has_user_reacted(&state.db.pg, message_id, user_id, &rc.emoji)
            .await
            .unwrap_or(false)
        {
            my_reactions.push(rc.emoji.clone());
        }
    }
    my_reactions
}
