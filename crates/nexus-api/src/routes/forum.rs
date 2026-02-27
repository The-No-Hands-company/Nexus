//! Forum channel routes — posts (threads), tags, lock/unlock.
//!
//! Forum channels are structured thread boards where every conversation is a
//! titled post with optional tags, rather than a flat chat flow.
//!
//! POST   /channels/:id/posts                      — Create forum post
//! GET    /channels/:id/posts                      — List forum posts (paginated)
//! PATCH  /channels/:id/posts/:thread_id           — Edit post title / tags (OP or MANAGE_THREADS)
//! POST   /channels/:id/posts/:thread_id/lock      — Lock post (MANAGE_THREADS)
//! DELETE /channels/:id/posts/:thread_id/lock      — Unlock post (MANAGE_THREADS)
//! GET    /channels/:id/tags                       — List forum tags
//! POST   /channels/:id/tags                       — Create forum tag (MANAGE_CHANNELS)
//! PATCH  /channels/:id/tags/:tag_id               — Update forum tag (MANAGE_CHANNELS)
//! DELETE /channels/:id/tags/:tag_id               — Delete forum tag (MANAGE_CHANNELS)

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::GatewayEvent,
    snowflake,
};
use nexus_db::repository::{channels, members, roles, servers, threads};
use nexus_common::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// ============================================================
// Router
// ============================================================

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Posts (forum threads)
        .route(
            "/channels/{channel_id}/posts",
            get(list_posts).post(create_post),
        )
        .route(
            "/channels/{channel_id}/posts/{thread_id}",
            axum::routing::patch(update_post),
        )
        .route(
            "/channels/{channel_id}/posts/{thread_id}/lock",
            post(lock_post).delete(unlock_post),
        )
        // Tags
        .route(
            "/channels/{channel_id}/tags",
            get(list_tags).post(create_tag),
        )
        .route(
            "/channels/{channel_id}/tags/{tag_id}",
            axum::routing::patch(update_tag).delete(delete_tag),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ============================================================
// Models
// ============================================================

#[derive(Debug, Deserialize)]
struct ListPostsQuery {
    /// Maximum number of posts to return (default 50, max 100)
    limit: Option<i64>,
    /// Pagination cursor: return posts before this thread ID
    before: Option<Uuid>,
    /// Filter by tag name
    tag: Option<String>,
    /// Include archived / closed posts (default: false)
    archived: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CreatePostRequest {
    /// Post title (required for forum posts)
    title: String,
    /// Initial message content
    content: String,
    /// Tags to apply to this post (tag names from the channel's tag list)
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct UpdatePostRequest {
    /// New title (optional)
    title: Option<String>,
    /// Replace tags list (optional)
    tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForumTag {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub name: String,
    pub emoji: Option<String>,
    pub moderated: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateTagRequest {
    name: String,
    emoji: Option<String>,
    moderated: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpdateTagRequest {
    name: Option<String>,
    emoji: Option<String>,
    moderated: Option<bool>,
}

// ============================================================
// Permission helpers
// ============================================================

/// Verify the caller has a required permission in the server that owns `channel_id`.
async fn require_channel_permission(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
    required: Permissions,
) -> NexusResult<Uuid> {
    let channel = channels::find_by_id(&state.db.pool, channel_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

    let server_id = channel.server_id.ok_or(NexusError::MissingPermission {
        permission: format!("{required:?}"),
    })?;

    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Server".into() })?;

    if user_id == server.owner_id {
        return Ok(server_id);
    }

    let member = members::find_member(&state.db.pool, user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or(NexusError::MissingPermission { permission: format!("{required:?}") })?;

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
        .fold(base, |acc, rp| acc | rp);

    if effective.has(required) {
        Ok(server_id)
    } else {
        Err(NexusError::MissingPermission { permission: format!("{required:?}") })
    }
}

// ============================================================
// Post handlers
// ============================================================

/// GET /api/v1/channels/:channel_id/posts
async fn list_posts(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Query(q): Query<ListPostsQuery>,
) -> NexusResult<Json<serde_json::Value>> {
    // Verify this is a forum channel
    let channel = channels::find_by_id(&state.db.pool, channel_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

    if !matches!(channel.channel_type, nexus_common::models::channel::ChannelType::Forum) {
        return Err(NexusError::Validation {
            message: "Channel is not a forum channel".into(),
        });
    }

    let limit = q.limit.unwrap_or(50).min(100);
    let include_archived = q.archived.unwrap_or(false);

    // Build query — forum posts are threads with this channel as parent
    let rows = if include_archived {
        threads::list_archived(&state.db.pool, channel_id, limit, None).await
    } else {
        threads::list_active(&state.db.pool, channel_id, limit).await
    }
    .map_err(|e| NexusError::Internal(e.into()))?;

    // Apply optional tag filter
    let tag_filter = q.tag.as_deref();
    let mut posts: Vec<serde_json::Value> = rows
        .into_iter()
        .filter(|row| {
            if let Some(tag) = tag_filter {
                row.tags.iter().any(|t| t == tag)
            } else {
                true
            }
        })
        .filter(|row| {
            if let Some(before_id) = q.before {
                row.channel_id < before_id
            } else {
                true
            }
        })
        .map(|row| {
            serde_json::json!({
                "id": row.channel_id,
                "parent_channel_id": row.parent_channel_id,
                "owner_id": row.owner_id,
                "title": row.title,
                "message_count": row.message_count,
                "member_count": row.member_count,
                "tags": row.tags,
                "archived": row.archived,
                "locked": row.locked,
                "auto_archive_minutes": row.auto_archive_minutes,
                "created_at": row.created_at,
                "updated_at": row.updated_at,
            })
        })
        .collect();

    // Honour before-cursor ordering (already sorted by DB, just truncate)
    posts.truncate(limit as usize);

    Ok(Json(serde_json::json!({ "posts": posts, "has_more": posts.len() == limit as usize })))
}

/// POST /api/v1/channels/:channel_id/posts
async fn create_post(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreatePostRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    if body.title.trim().is_empty() || body.title.len() > 100 {
        return Err(NexusError::Validation {
            message: "Post title must be 1-100 characters".into(),
        });
    }
    if body.content.trim().is_empty() || body.content.len() > 4000 {
        return Err(NexusError::Validation {
            message: "Post content must be 1-4000 characters".into(),
        });
    }

    let channel = channels::find_by_id(&state.db.pool, channel_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

    if !matches!(channel.channel_type, nexus_common::models::channel::ChannelType::Forum) {
        return Err(NexusError::Validation {
            message: "Channel is not a forum channel".into(),
        });
    }

    let tags = body.tags.clone().unwrap_or_default();

    // Validate tag names against the channel's tag list
    if !tags.is_empty() {
        let defined_tags = fetch_channel_tags(&state.db.pool, channel_id).await?;
        let defined_names: Vec<&str> = defined_tags.iter().map(|t| t.name.as_str()).collect();
        for tag in &tags {
            if !defined_names.contains(&tag.as_str()) {
                return Err(NexusError::Validation {
                    message: format!("Unknown forum tag: {tag}"),
                });
            }
        }
    }

    // Create a sub-channel for the forum post (type = thread, parent = forum channel)
    let thread_channel_id = snowflake::generate_id();
    channels::create_channel(
        &state.db.pool,
        thread_channel_id,
        channel.server_id,
        Some(channel_id),
        "thread",
        Some(&body.title),
        None,
        0,
    )
    .await?;

    // Create the thread record in the threads table
    let thread_row = threads::create_thread(
        &state.db.pool,
        thread_channel_id,
        channel_id,
        None, // forum posts don't have a parent message
        auth.user_id,
        &body.title,
        10080, // default: 7 days auto-archive
        &tags,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    // Post the opening message into the thread channel
    let msg_id = snowflake::generate_id();
    sqlx::query(
        "INSERT INTO messages (id, channel_id, author_id, content, created_at, updated_at) \
         VALUES ($1::uuid, $2::uuid, $3::uuid, $4, NOW(), NOW())",
    )
    .bind(msg_id.to_string())
    .bind(thread_channel_id.to_string())
    .bind(auth.user_id.to_string())
    .bind(&body.content)
    .execute(&state.db.pool)
    .await?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: nexus_common::gateway_event::event_types::FORUM_POST_CREATE.to_string(),
        data: serde_json::json!({
            "post_id": thread_channel_id,
            "channel_id": channel_id,
            "owner_id": auth.user_id,
            "title": body.title,
            "tags": tags,
        }),
        server_id: channel.server_id,
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({
        "id": thread_row.channel_id,
        "parent_channel_id": channel_id,
        "owner_id": thread_row.owner_id,
        "title": thread_row.title,
        "message_count": thread_row.message_count,
        "member_count": thread_row.member_count,
        "tags": thread_row.tags,
        "archived": thread_row.archived,
        "locked": thread_row.locked,
        "auto_archive_minutes": thread_row.auto_archive_minutes,
        "created_at": thread_row.created_at,
        "updated_at": thread_row.updated_at,
    })))
}

/// PATCH /api/v1/channels/:channel_id/posts/:thread_id
async fn update_post(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, thread_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdatePostRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    // Thread must exist and belong to this forum channel
    let thread = threads::find_by_id(&state.db.pool, thread_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or(NexusError::NotFound { resource: "Post".into() })?;

    if thread.parent_channel_id.unwrap_or(thread.channel_id) != channel_id {
        return Err(NexusError::NotFound { resource: "Post".into() });
    }

    // Only the post owner or someone with MANAGE_THREADS may edit
    let is_owner = thread.owner_id == auth.user_id;
    if !is_owner {
        require_channel_permission(&state, channel_id, auth.user_id, Permissions::MANAGE_THREADS)
            .await?;
    }

    if let Some(title) = &body.title {
        if title.trim().is_empty() || title.len() > 100 {
            return Err(NexusError::Validation {
                message: "Post title must be 1-100 characters".into(),
            });
        }
    }

    // Validate tags if provided
    let tags = if let Some(ref new_tags) = body.tags {
        let defined_tags = fetch_channel_tags(&state.db.pool, channel_id).await?;
        let defined_names: Vec<&str> = defined_tags.iter().map(|t| t.name.as_str()).collect();
        for tag in new_tags {
            if !defined_names.contains(&tag.as_str()) {
                return Err(NexusError::Validation {
                    message: format!("Unknown forum tag: {tag}"),
                });
            }
        }
        new_tags.clone()
    } else {
        thread.tags.clone()
    };

    let title = body.title.as_deref().unwrap_or(&thread.title);
    let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());

    sqlx::query(
        "UPDATE threads SET title = $1, tags = $2, updated_at = NOW() WHERE channel_id = $3::uuid",
    )
    .bind(title)
    .bind(&tags_json)
    .bind(thread_id.to_string())
    .execute(&state.db.pool)
    .await?;

    // Also update the channel name (it mirrors the thread title)
    sqlx::query("UPDATE channels SET name = $1, updated_at = NOW() WHERE id = $2::uuid")
        .bind(title)
        .bind(thread_id.to_string())
        .execute(&state.db.pool)
        .await?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: nexus_common::gateway_event::event_types::FORUM_POST_UPDATE.to_string(),
        data: serde_json::json!({
            "post_id": thread_id,
            "channel_id": channel_id,
            "title": title,
            "tags": tags,
        }),
        server_id: None,
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({
        "id": thread_id,
        "parent_channel_id": channel_id,
        "owner_id": thread.owner_id,
        "title": title,
        "tags": tags,
        "archived": thread.archived,
        "locked": thread.locked,
    })))
}

/// POST /api/v1/channels/:channel_id/posts/:thread_id/lock
async fn lock_post(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, thread_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    require_channel_permission(&state, channel_id, auth.user_id, Permissions::MANAGE_THREADS)
        .await?;

    sqlx::query(
        "UPDATE threads SET locked = true, updated_at = NOW() WHERE channel_id = $1::uuid",
    )
    .bind(thread_id.to_string())
    .execute(&state.db.pool)
    .await?;

    Ok(Json(serde_json::json!({ "locked": true, "post_id": thread_id })))
}

/// DELETE /api/v1/channels/:channel_id/posts/:thread_id/lock
async fn unlock_post(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, thread_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    require_channel_permission(&state, channel_id, auth.user_id, Permissions::MANAGE_THREADS)
        .await?;

    sqlx::query(
        "UPDATE threads SET locked = false, updated_at = NOW() WHERE channel_id = $1::uuid",
    )
    .bind(thread_id.to_string())
    .execute(&state.db.pool)
    .await?;

    Ok(Json(serde_json::json!({ "locked": false, "post_id": thread_id })))
}

// ============================================================
// Tag handlers
// ============================================================

/// GET /api/v1/channels/:channel_id/tags
async fn list_tags(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Vec<ForumTag>>> {
    let tags = fetch_channel_tags(&state.db.pool, channel_id).await?;
    Ok(Json(tags))
}

/// POST /api/v1/channels/:channel_id/tags
async fn create_tag(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateTagRequest>,
) -> NexusResult<Json<ForumTag>> {
    if body.name.trim().is_empty() || body.name.len() > 20 {
        return Err(NexusError::Validation {
            message: "Tag name must be 1-20 characters".into(),
        });
    }

    require_channel_permission(&state, channel_id, auth.user_id, Permissions::MANAGE_CHANNELS)
        .await?;

    let tag_id = Uuid::new_v4();
    let moderated = body.moderated.unwrap_or(false);

    sqlx::query(
        "INSERT INTO forum_tags (id, channel_id, name, emoji, moderated) \
         VALUES ($1::uuid, $2::uuid, $3, $4, $5)",
    )
    .bind(tag_id.to_string())
    .bind(channel_id.to_string())
    .bind(&body.name)
    .bind(body.emoji.as_deref())
    .bind(moderated)
    .execute(&state.db.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
            NexusError::Validation { message: format!("Tag '{}' already exists", body.name) }
        } else {
            NexusError::Internal(e.into())
        }
    })?;

    Ok(Json(ForumTag {
        id: tag_id,
        channel_id,
        name: body.name,
        emoji: body.emoji,
        moderated,
        created_at: Utc::now(),
    }))
}

/// PATCH /api/v1/channels/:channel_id/tags/:tag_id
async fn update_tag(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, tag_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateTagRequest>,
) -> NexusResult<Json<ForumTag>> {
    require_channel_permission(&state, channel_id, auth.user_id, Permissions::MANAGE_CHANNELS)
        .await?;

    if let Some(ref name) = body.name {
        if name.trim().is_empty() || name.len() > 20 {
            return Err(NexusError::Validation {
                message: "Tag name must be 1-20 characters".into(),
            });
        }
    }

    #[derive(sqlx::FromRow)]
    struct TagRow {
        id: String,
        channel_id: String,
        name: String,
        emoji: Option<String>,
        moderated: bool,
        created_at: String,
    }

    let existing: Option<TagRow> = sqlx::query_as(
        "SELECT id::text, channel_id::text, name, emoji, moderated, created_at::text AS created_at \
         FROM forum_tags WHERE id = $1::uuid AND channel_id = $2::uuid",
    )
    .bind(tag_id.to_string())
    .bind(channel_id.to_string())
    .fetch_optional(&state.db.pool)
    .await?;

    let existing = existing.ok_or(NexusError::NotFound { resource: "Tag".into() })?;

    let new_name = body.name.as_deref().unwrap_or(&existing.name);
    let new_emoji = body.emoji.as_deref().or(existing.emoji.as_deref());
    let new_moderated = body.moderated.unwrap_or(existing.moderated);

    sqlx::query(
        "UPDATE forum_tags SET name = $1, emoji = $2, moderated = $3 \
         WHERE id = $4::uuid AND channel_id = $5::uuid",
    )
    .bind(new_name)
    .bind(new_emoji)
    .bind(new_moderated)
    .bind(tag_id.to_string())
    .bind(channel_id.to_string())
    .execute(&state.db.pool)
    .await?;

    Ok(Json(ForumTag {
        id: tag_id,
        channel_id,
        name: new_name.to_string(),
        emoji: new_emoji.map(str::to_string),
        moderated: new_moderated,
        created_at: parse_dt(&existing.created_at)
            .unwrap_or_else(|_| chrono::Utc::now()),
    }))
}

/// DELETE /api/v1/channels/:channel_id/tags/:tag_id
async fn delete_tag(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, tag_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    require_channel_permission(&state, channel_id, auth.user_id, Permissions::MANAGE_CHANNELS)
        .await?;

    sqlx::query("DELETE FROM forum_tags WHERE id = $1::uuid AND channel_id = $2::uuid")
        .bind(tag_id.to_string())
        .bind(channel_id.to_string())
        .execute(&state.db.pool)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ============================================================
// Internal helpers
// ============================================================

/// Fetch the tag list for a forum channel from the DB.
async fn fetch_channel_tags(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
) -> NexusResult<Vec<ForumTag>> {
    #[derive(sqlx::FromRow)]
    struct TagRow {
        id: String,
        channel_id: String,
        name: String,
        emoji: Option<String>,
        moderated: bool,
        created_at: String,
    }

    let rows: Vec<TagRow> = sqlx::query_as(
        "SELECT id::text, channel_id::text, name, emoji, moderated, created_at::text AS created_at \
         FROM forum_tags WHERE channel_id = $1::uuid ORDER BY name ASC",
    )
    .bind(channel_id.to_string())
    .fetch_all(pool)
    .await?;

    let tags = rows
        .into_iter()
        .map(|r| ForumTag {
            id: r.id.parse().unwrap_or(Uuid::nil()),
            channel_id: r.channel_id.parse().unwrap_or(Uuid::nil()),
            name: r.name,
            emoji: r.emoji,
            moderated: r.moderated,
            created_at: parse_dt(&r.created_at).unwrap_or_else(|_| chrono::Utc::now()),
        })
        .collect();

    Ok(tags)
}

/// Parse a Postgres/SQLite timestamp text representation into `DateTime<Utc>`.
fn parse_dt(s: &str) -> Result<DateTime<Utc>, String> {
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Ok(d.with_timezone(&Utc));
    }
    // Postgres TIMESTAMPTZ::text: "2026-02-22 13:04:47.779907+00"
    let iso = s.replacen(' ', "T", 1);
    let iso = {
        let len = iso.len();
        if len >= 3 {
            let last3 = &iso[len - 3..];
            if (last3.starts_with('+') || last3.starts_with('-'))
                && last3[1..].chars().all(|c| c.is_ascii_digit())
            {
                format!("{}:00", iso)
            } else {
                iso
            }
        } else {
            iso
        }
    };
    if let Ok(d) = DateTime::parse_from_rfc3339(&iso) {
        return Ok(d.with_timezone(&Utc));
    }
    if let Ok(d) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(d.and_utc());
    }
    if let Ok(d) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
        return Ok(d.and_utc());
    }
    Err(format!("cannot parse timestamp '{s}'"))
}
