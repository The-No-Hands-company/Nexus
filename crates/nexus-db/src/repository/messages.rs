//! Message repository — CRUD operations for messages.
//!
//! Supports both PostgreSQL (production) and SQLite (lite mode) via AnyPool.
//! All UUID/DateTime/Vec<Uuid>/JSON fields are encoded as strings for AnyPool compatibility.

use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

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
    let mentions_json = serde_json::to_string(
        &mentions.iter().map(|x| x.to_string()).collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".to_string());
    let mention_roles_json = serde_json::to_string(
        &mention_roles.iter().map(|x| x.to_string()).collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".to_string());

    sqlx::query_as::<_, MessageRow>(
        r#"
        INSERT INTO messages (
            id, channel_id, author_id, content, message_type,
            edited, pinned, embeds, attachments,
            mentions, mention_roles, mention_everyone,
            reference_message_id, reference_channel_id,
            flags, created_at, updated_at
        )
        VALUES (
            ?, ?, ?, ?, ?,
            false, false, '[]', '[]',
            ?, ?, ?,
            ?, ?,
            0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        )
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(channel_id.to_string())
    .bind(author_id.to_string())
    .bind(content)
    .bind(message_type)
    .bind(&mentions_json)
    .bind(&mention_roles_json)
    .bind(mention_everyone)
    .bind(reference_message_id.map(|x| x.to_string()))
    .bind(reference_channel_id.map(|x| x.to_string()))
    .fetch_one(pool)
    .await
}

/// Find a message by ID.
pub async fn find_by_id(pool: &sqlx::AnyPool, id: Uuid) -> Result<Option<MessageRow>, sqlx::Error> {
    sqlx::query_as::<_, MessageRow>("SELECT * FROM messages WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
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
        sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT m.* FROM messages m
            WHERE m.channel_id = ?
              AND m.created_at < (SELECT created_at FROM messages WHERE id = ?)
            ORDER BY m.created_at DESC
            LIMIT ?
            "#,
        )
        .bind(channel_id.to_string())
        .bind(before_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    } else if let Some(after_id) = after {
        sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT * FROM (
                SELECT m.* FROM messages m
                WHERE m.channel_id = ?
                  AND m.created_at > (SELECT created_at FROM messages WHERE id = ?)
                ORDER BY m.created_at ASC
                LIMIT ?
            ) sub ORDER BY created_at DESC
            "#,
        )
        .bind(channel_id.to_string())
        .bind(after_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT * FROM messages
            WHERE channel_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
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
        sqlx::query_as::<_, MessageWithAuthor>(
            r#"
            SELECT m.*, u.username AS author_username
            FROM messages m
            JOIN users u ON u.id = m.author_id
            WHERE m.channel_id = ?
              AND m.created_at < (SELECT created_at FROM messages WHERE id = ?)
            ORDER BY m.created_at DESC
            LIMIT ?
            "#,
        )
        .bind(channel_id.to_string())
        .bind(before_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    } else if let Some(after_id) = after {
        sqlx::query_as::<_, MessageWithAuthor>(
            r#"
            SELECT * FROM (
                SELECT m.*, u.username AS author_username
                FROM messages m
                JOIN users u ON u.id = m.author_id
                WHERE m.channel_id = ?
                  AND m.created_at > (SELECT created_at FROM messages WHERE id = ?)
                ORDER BY m.created_at ASC
                LIMIT ?
            ) sub ORDER BY created_at DESC
            "#,
        )
        .bind(channel_id.to_string())
        .bind(after_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, MessageWithAuthor>(
            r#"
            SELECT m.*, u.username AS author_username
            FROM messages m
            JOIN users u ON u.id = m.author_id
            WHERE m.channel_id = ?
            ORDER BY m.created_at DESC
            LIMIT ?
            "#,
        )
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
    sqlx::query_as::<_, MessageRow>(
        r#"
        UPDATE messages SET
            content = ?,
            edited = true,
            edited_at = CURRENT_TIMESTAMP,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        RETURNING *
        "#,
    )
    .bind(content)
    .bind(id.to_string())
    .fetch_one(pool)
    .await
}

/// Delete a single message.
pub async fn delete_message(pool: &sqlx::AnyPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM messages WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Bulk delete messages (for moderation). Returns count deleted.
pub async fn bulk_delete_messages(pool: &sqlx::AnyPool, ids: &[Uuid]) -> Result<u64, sqlx::Error> {
    let mut total: u64 = 0;
    for id in ids {
        let result = sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(id.to_string())
            .execute(pool)
            .await?;
        total += result.rows_affected();
    }
    Ok(total)
}

/// Pin a message.
pub async fn pin_message(pool: &sqlx::AnyPool, id: Uuid) -> Result<MessageRow, sqlx::Error> {
    sqlx::query_as::<_, MessageRow>(
        "UPDATE messages SET pinned = true, updated_at = CURRENT_TIMESTAMP WHERE id = ? RETURNING *",
    )
    .bind(id.to_string())
    .fetch_one(pool)
    .await
}

/// Unpin a message.
pub async fn unpin_message(pool: &sqlx::AnyPool, id: Uuid) -> Result<MessageRow, sqlx::Error> {
    sqlx::query_as::<_, MessageRow>(
        "UPDATE messages SET pinned = false, updated_at = CURRENT_TIMESTAMP WHERE id = ? RETURNING *",
    )
    .bind(id.to_string())
    .fetch_one(pool)
    .await
}

/// Get pinned messages in a channel.
pub async fn get_pinned_messages(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
) -> Result<Vec<MessageRow>, sqlx::Error> {
    sqlx::query_as::<_, MessageRow>(
        "SELECT * FROM messages WHERE channel_id = ? AND pinned = true ORDER BY created_at DESC",
    )
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
        sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT * FROM messages
            WHERE channel_id = ?
              AND search_vector @@ plainto_tsquery('english', ?)
            ORDER BY ts_rank(search_vector, plainto_tsquery('english', ?)) DESC, created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(cid.to_string())
        .bind(query)
        .bind(query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT * FROM messages
            WHERE search_vector @@ plainto_tsquery('english', ?)
            ORDER BY ts_rank(search_vector, plainto_tsquery('english', ?)) DESC, created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
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
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages WHERE channel_id = ?")
        .bind(channel_id.to_string())
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}
