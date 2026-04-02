//! Message repository — CRUD operations for messages.
//!
//! Supports both PostgreSQL (production) and SQLite (lite mode) via AnyPool.
//! All UUID/DateTime/Vec<Uuid>/JSON fields are encoded as strings for AnyPool compatibility.

use chrono::{DateTime, Utc};
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

use crate::select_cols::{MESSAGE_COLS, MESSAGE_COLS_M};

/// Row type for messages.
/// Implements FromRow manually to handle AnyPool (Uuid/DateTime as strings).
#[derive(Debug)]
pub struct MessageRow {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub message_type: i32,
    pub edited: bool,
    pub edited_at: Option<DateTime<Utc>>,
    pub pinned: bool,
    pub embeds: serde_json::Value,
    pub attachments: serde_json::Value,
    pub mentions: Vec<Uuid>,
    pub mention_roles: Vec<Uuid>,
    pub mention_everyone: bool,
    pub reference_message_id: Option<Uuid>,
    pub reference_channel_id: Option<Uuid>,
    pub thread_id: Option<Uuid>,
    pub flags: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> for MessageRow {
    fn from_row(row: &'r sqlx::any::AnyRow) -> Result<Self, sqlx::Error> {
        use crate::any_compat::*;
        Ok(MessageRow {
            id: get_uuid(row, "id")?,
            channel_id: get_uuid(row, "channel_id")?,
            author_id: get_uuid(row, "author_id")?,
            content: row.try_get("content")?,
            message_type: row.try_get("message_type")?,
            edited: row.try_get("edited")?,
            edited_at: get_opt_datetime(row, "edited_at")?,
            pinned: row.try_get("pinned")?,
            embeds: get_json_value(row, "embeds")?,
            attachments: get_json_value(row, "attachments")?,
            mentions: get_uuid_vec(row, "mentions")?,
            mention_roles: get_uuid_vec(row, "mention_roles")?,
            mention_everyone: row.try_get("mention_everyone")?,
            reference_message_id: get_opt_uuid(row, "reference_message_id")?,
            reference_channel_id: get_opt_uuid(row, "reference_channel_id")?,
            thread_id: get_opt_uuid(row, "thread_id")?,
            flags: row.try_get("flags")?,
            created_at: get_datetime(row, "created_at")?,
            updated_at: get_datetime(row, "updated_at")?,
        })
    }
}

/// Create a new message.
pub async fn create_message(
    pool: &sqlx::AnyPool,
    id: Uuid,
    channel_id: Uuid,
    author_id: Uuid,
    content: &str,
    message_type: i32,
    reference_message_id: Option<Uuid>,
    reference_channel_id: Option<Uuid>,
    mentions: &[Uuid],
    mention_roles: &[Uuid],
    mention_everyone: bool,
) -> Result<MessageRow, sqlx::Error> {
    // Build Postgres array literals: '{}' for empty, '{uuid1,uuid2}' for populated.
    let mentions_arr = format!("{{{}}}", mentions.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
    let mention_roles_arr = format!("{{{}}}", mention_roles.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));

    let q = format!(
        "INSERT INTO messages ( \
             id, channel_id, author_id, content, message_type, \
             edited, pinned, embeds, attachments, \
             mentions, mention_roles, mention_everyone, \
             reference_message_id, reference_channel_id, \
             flags, created_at, updated_at \
         ) VALUES ( \
             $1::uuid, $2::uuid, $3::uuid, $4, $5, \
             false, false, '[]'::jsonb, '[]'::jsonb, \
             $6::uuid[], $7::uuid[], $8, \
             $9::uuid, $10::uuid, \
             0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP \
         ) RETURNING {MESSAGE_COLS}"
    );
    sqlx::query_as::<_, MessageRow>(&q)
    .bind(id.to_string())
    .bind(channel_id.to_string())
    .bind(author_id.to_string())
    .bind(content)
    .bind(message_type)
    .bind(&mentions_arr)
    .bind(&mention_roles_arr)
    .bind(mention_everyone)
    .bind(reference_message_id.map(|x| x.to_string()))
    .bind(reference_channel_id.map(|x| x.to_string()))
    .fetch_one(pool)
    .await
}

/// Find a message by ID.
pub async fn find_by_id(pool: &sqlx::AnyPool, id: Uuid) -> Result<Option<MessageRow>, sqlx::Error> {
    let q = format!("SELECT {MESSAGE_COLS} FROM messages WHERE id = $1::uuid");
    sqlx::query_as::<_, MessageRow>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

/// Batch-load messages by ID.
///
/// Returns a map keyed by message ID for quick lookups in API assembly code.
pub async fn find_by_ids_map(
    pool: &sqlx::AnyPool,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, MessageRow>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let id_strings: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    let mut qb = sqlx::QueryBuilder::<sqlx::Any>::new("SELECT ");
    qb.push(MESSAGE_COLS);
    qb.push(" FROM messages WHERE id IN (");

    {
        let mut separated = qb.separated(", ");
        for id in &id_strings {
            separated.push_bind(id);
        }
    }
    qb.push(")");

    let rows = qb
        .build_query_as::<MessageRow>()
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

/// List messages in a channel with cursor-based pagination.
///
/// - `before`: Get messages before this ID (older)
/// - `after`: Get messages after this ID (newer)
/// - `limit`: Max messages to return (default 50, max 100)
///
/// Returns messages in reverse chronological order (newest first).
pub async fn list_channel_messages(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
    before: Option<Uuid>,
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<MessageRow>, sqlx::Error> {
    let limit = limit.min(100).max(1);

    if let Some(before_id) = before {
        let q = format!(
            "SELECT {MESSAGE_COLS_M} FROM messages m \
             WHERE m.channel_id = $1::uuid \
               AND m.created_at < (SELECT created_at FROM messages WHERE id = $2::uuid) \
             ORDER BY m.created_at DESC \
             LIMIT $3"
        );
        sqlx::query_as::<_, MessageRow>(&q)
        .bind(channel_id.to_string())
        .bind(before_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    } else if let Some(after_id) = after {
        let q = format!(
            "SELECT * FROM ( \
                 SELECT {MESSAGE_COLS_M} FROM messages m \
                 WHERE m.channel_id = $1::uuid \
                   AND m.created_at > (SELECT created_at FROM messages WHERE id = $2::uuid) \
                 ORDER BY m.created_at ASC \
                 LIMIT $3 \
             ) sub ORDER BY created_at DESC"
        );
        sqlx::query_as::<_, MessageRow>(&q)
        .bind(channel_id.to_string())
        .bind(after_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        let q = format!(
            "SELECT {MESSAGE_COLS} FROM messages \
             WHERE channel_id = $1::uuid \
             ORDER BY created_at DESC \
             LIMIT $2"
        );
        sqlx::query_as::<_, MessageRow>(&q)
        .bind(channel_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

/// Message row with author username (via JOIN with users table).
#[derive(Debug)]
pub struct MessageWithAuthor {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub content: String,
    pub message_type: i32,
    pub edited: bool,
    pub edited_at: Option<DateTime<Utc>>,
    pub pinned: bool,
    pub embeds: serde_json::Value,
    pub attachments: serde_json::Value,
    pub mentions: Vec<Uuid>,
    pub mention_roles: Vec<Uuid>,
    pub mention_everyone: bool,
    pub reference_message_id: Option<Uuid>,
    pub reference_channel_id: Option<Uuid>,
    pub thread_id: Option<Uuid>,
    pub flags: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> for MessageWithAuthor {
    fn from_row(row: &'r sqlx::any::AnyRow) -> Result<Self, sqlx::Error> {
        use crate::any_compat::*;
        Ok(MessageWithAuthor {
            id: get_uuid(row, "id")?,
            channel_id: get_uuid(row, "channel_id")?,
            author_id: get_uuid(row, "author_id")?,
            author_username: row.try_get("author_username")?,
            content: row.try_get("content")?,
            message_type: row.try_get("message_type")?,
            edited: row.try_get("edited")?,
            edited_at: get_opt_datetime(row, "edited_at")?,
            pinned: row.try_get("pinned")?,
            embeds: get_json_value(row, "embeds")?,
            attachments: get_json_value(row, "attachments")?,
            mentions: get_uuid_vec(row, "mentions")?,
            mention_roles: get_uuid_vec(row, "mention_roles")?,
            mention_everyone: row.try_get("mention_everyone")?,
            reference_message_id: get_opt_uuid(row, "reference_message_id")?,
            reference_channel_id: get_opt_uuid(row, "reference_channel_id")?,
            thread_id: get_opt_uuid(row, "thread_id")?,
            flags: row.try_get("flags")?,
            created_at: get_datetime(row, "created_at")?,
            updated_at: get_datetime(row, "updated_at")?,
        })
    }
}

/// List messages in a channel with author usernames (JOIN users), cursor-based pagination.
pub async fn list_channel_messages_with_author(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
    before: Option<Uuid>,
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<MessageWithAuthor>, sqlx::Error> {
    let limit = limit.min(100).max(1);
    if let Some(before_id) = before {
        let q = format!(
            "SELECT {MESSAGE_COLS_M}, u.username AS author_username \
             FROM messages m \
             JOIN users u ON u.id = m.author_id \
             WHERE m.channel_id = $1::uuid \
               AND m.created_at < (SELECT created_at FROM messages WHERE id = $2::uuid) \
             ORDER BY m.created_at DESC \
             LIMIT $3"
        );
        sqlx::query_as::<_, MessageWithAuthor>(&q)
        .bind(channel_id.to_string())
        .bind(before_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    } else if let Some(after_id) = after {
        let q = format!(
            "SELECT * FROM ( \
                 SELECT {MESSAGE_COLS_M}, u.username AS author_username \
                 FROM messages m \
                 JOIN users u ON u.id = m.author_id \
                 WHERE m.channel_id = $1::uuid \
                   AND m.created_at > (SELECT created_at FROM messages WHERE id = $2::uuid) \
                 ORDER BY m.created_at ASC \
                 LIMIT $3 \
             ) sub ORDER BY created_at DESC"
        );
        sqlx::query_as::<_, MessageWithAuthor>(&q)
        .bind(channel_id.to_string())
        .bind(after_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        let q = format!(
            "SELECT {MESSAGE_COLS_M}, u.username AS author_username \
             FROM messages m \
             JOIN users u ON u.id = m.author_id \
             WHERE m.channel_id = $1::uuid \
             ORDER BY m.created_at DESC \
             LIMIT $2"
        );
        sqlx::query_as::<_, MessageWithAuthor>(&q)
        .bind(channel_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

/// Update a message's content (edit).
pub async fn update_message(
    pool: &sqlx::AnyPool,
    id: Uuid,
    content: &str,
) -> Result<MessageRow, sqlx::Error> {
    let q = format!(
        "UPDATE messages SET \
             content = $1, edited = true, \
             edited_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
         WHERE id = $2::uuid \
         RETURNING {MESSAGE_COLS}"
    );
    sqlx::query_as::<_, MessageRow>(&q)
    .bind(content)
    .bind(id.to_string())
    .fetch_one(pool)
    .await
}

/// Delete a single message.
pub async fn delete_message(pool: &sqlx::AnyPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM messages WHERE id = $1::uuid")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Bulk delete messages (for moderation). Returns count deleted.
pub async fn bulk_delete_messages(pool: &sqlx::AnyPool, ids: &[Uuid]) -> Result<u64, sqlx::Error> {
    let mut total: u64 = 0;
    for id in ids {
        let result = sqlx::query("DELETE FROM messages WHERE id = $1::uuid")
            .bind(id.to_string())
            .execute(pool)
            .await?;
        total += result.rows_affected();
    }
    Ok(total)
}

/// Pin a message.
pub async fn pin_message(pool: &sqlx::AnyPool, id: Uuid) -> Result<MessageRow, sqlx::Error> {
    let q = format!("UPDATE messages SET pinned = true, updated_at = CURRENT_TIMESTAMP WHERE id = $1::uuid RETURNING {MESSAGE_COLS}");
    sqlx::query_as::<_, MessageRow>(&q)
    .bind(id.to_string())
    .fetch_one(pool)
    .await
}

/// Unpin a message.
pub async fn unpin_message(pool: &sqlx::AnyPool, id: Uuid) -> Result<MessageRow, sqlx::Error> {
    let q = format!("UPDATE messages SET pinned = false, updated_at = CURRENT_TIMESTAMP WHERE id = $1::uuid RETURNING {MESSAGE_COLS}");
    sqlx::query_as::<_, MessageRow>(&q)
    .bind(id.to_string())
    .fetch_one(pool)
    .await
}

/// Get pinned messages in a channel.
pub async fn get_pinned_messages(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
) -> Result<Vec<MessageRow>, sqlx::Error> {
    let q = format!("SELECT {MESSAGE_COLS} FROM messages WHERE channel_id = $1::uuid AND pinned = true ORDER BY created_at DESC");
    sqlx::query_as::<_, MessageRow>(&q)
    .bind(channel_id.to_string())
    .fetch_all(pool)
    .await
}

/// Search messages using full-text search (PostgreSQL only; returns empty for SQLite).
pub async fn search_messages(
    pool: &sqlx::AnyPool,
    channel_id: Option<Uuid>,
    query: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<MessageRow>, sqlx::Error> {
    let limit = limit.min(50).max(1);

    if let Some(cid) = channel_id {
        let q = format!(
            "SELECT {MESSAGE_COLS} FROM messages \
             WHERE channel_id = $1::uuid \
               AND search_vector @@ plainto_tsquery('english', $2) \
             ORDER BY ts_rank(search_vector, plainto_tsquery('english', $3)) DESC, created_at DESC \
             LIMIT $4 OFFSET $5"
        );
        sqlx::query_as::<_, MessageRow>(&q)
        .bind(cid.to_string())
        .bind(query)
        .bind(query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    } else {
        let q = format!(
            "SELECT {MESSAGE_COLS} FROM messages \
             WHERE search_vector @@ plainto_tsquery('english', $1) \
             ORDER BY ts_rank(search_vector, plainto_tsquery('english', $2)) DESC, created_at DESC \
             LIMIT $3 OFFSET $4"
        );
        sqlx::query_as::<_, MessageRow>(&q)
        .bind(query)
        .bind(query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }
}

/// Count messages in a channel (for stats).
pub async fn count_channel_messages(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages WHERE channel_id = $1::uuid")
        .bind(channel_id.to_string())
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}
