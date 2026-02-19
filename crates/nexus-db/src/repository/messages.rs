//! Message repository — CRUD operations for messages.
//!
//! Messages are stored in PostgreSQL for the MVP.
//! Cursor-based pagination uses (created_at, id) for stable iteration.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Row type for messages from PostgreSQL.
/// We use a flat struct + manual mapping since Message model has nested types.
#[derive(Debug, sqlx::FromRow)]
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

/// Create a new message.
pub async fn create_message(
    pool: &PgPool,
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
            $1, $2, $3, $4, $5,
            false, false, '[]'::jsonb, '[]'::jsonb,
            $6, $7, $8,
            $9, $10,
            0, NOW(), NOW()
        )
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(channel_id)
    .bind(author_id)
    .bind(content)
    .bind(message_type)
    .bind(mentions)
    .bind(mention_roles)
    .bind(mention_everyone)
    .bind(reference_message_id)
    .bind(reference_channel_id)
    .fetch_one(pool)
    .await
}

/// Find a message by ID.
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<MessageRow>, sqlx::Error> {
    sqlx::query_as::<_, MessageRow>("SELECT * FROM messages WHERE id = $1")
        .bind(id)
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
    pool: &PgPool,
    channel_id: Uuid,
    before: Option<Uuid>,
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<MessageRow>, sqlx::Error> {
    let limit = limit.min(100).max(1);

    if let Some(before_id) = before {
        // Get messages older than the cursor
        sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT m.* FROM messages m
            WHERE m.channel_id = $1
              AND m.created_at < (SELECT created_at FROM messages WHERE id = $2)
            ORDER BY m.created_at DESC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(before_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else if let Some(after_id) = after {
        // Get messages newer than the cursor
        sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT * FROM (
                SELECT m.* FROM messages m
                WHERE m.channel_id = $1
                  AND m.created_at > (SELECT created_at FROM messages WHERE id = $2)
                ORDER BY m.created_at ASC
                LIMIT $3
            ) sub ORDER BY created_at DESC
            "#,
        )
        .bind(channel_id)
        .bind(after_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        // Get latest messages
        sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT * FROM messages
            WHERE channel_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(channel_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

/// Message row with author username (via JOIN with users table).
#[derive(Debug, sqlx::FromRow)]
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

/// List messages in a channel with author usernames (JOIN users), cursor-based pagination.
pub async fn list_channel_messages_with_author(
    pool: &PgPool,
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
            WHERE m.channel_id = $1
              AND m.created_at < (SELECT created_at FROM messages WHERE id = $2)
            ORDER BY m.created_at DESC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(before_id)
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
                WHERE m.channel_id = $1
                  AND m.created_at > (SELECT created_at FROM messages WHERE id = $2)
                ORDER BY m.created_at ASC
                LIMIT $3
            ) sub ORDER BY created_at DESC
            "#,
        )
        .bind(channel_id)
        .bind(after_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, MessageWithAuthor>(
            r#"
            SELECT m.*, u.username AS author_username
            FROM messages m
            JOIN users u ON u.id = m.author_id
            WHERE m.channel_id = $1
            ORDER BY m.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(channel_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

/// Update a message's content (edit).
pub async fn update_message(
    pool: &PgPool,
    id: Uuid,
    content: &str,
) -> Result<MessageRow, sqlx::Error> {
    sqlx::query_as::<_, MessageRow>(
        r#"
        UPDATE messages SET
            content = $2,
            edited = true,
            edited_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(content)
    .fetch_one(pool)
    .await
}

/// Delete a single message.
pub async fn delete_message(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM messages WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Bulk delete messages (for moderation). Returns count deleted.
pub async fn bulk_delete_messages(pool: &PgPool, ids: &[Uuid]) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM messages WHERE id = ANY($1)")
        .bind(ids)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Pin a message.
pub async fn pin_message(pool: &PgPool, id: Uuid) -> Result<MessageRow, sqlx::Error> {
    sqlx::query_as::<_, MessageRow>(
        "UPDATE messages SET pinned = true, updated_at = NOW() WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

/// Unpin a message.
pub async fn unpin_message(pool: &PgPool, id: Uuid) -> Result<MessageRow, sqlx::Error> {
    sqlx::query_as::<_, MessageRow>(
        "UPDATE messages SET pinned = false, updated_at = NOW() WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

/// Get pinned messages in a channel.
pub async fn get_pinned_messages(
    pool: &PgPool,
    channel_id: Uuid,
) -> Result<Vec<MessageRow>, sqlx::Error> {
    sqlx::query_as::<_, MessageRow>(
        "SELECT * FROM messages WHERE channel_id = $1 AND pinned = true ORDER BY created_at DESC",
    )
    .bind(channel_id)
    .fetch_all(pool)
    .await
}

/// Search messages using PostgreSQL full-text search.
pub async fn search_messages(
    pool: &PgPool,
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
            WHERE channel_id = $1
              AND search_vector @@ plainto_tsquery('english', $2)
            ORDER BY ts_rank(search_vector, plainto_tsquery('english', $2)) DESC, created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(cid)
        .bind(query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT * FROM messages
            WHERE search_vector @@ plainto_tsquery('english', $1)
            ORDER BY ts_rank(search_vector, plainto_tsquery('english', $1)) DESC, created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }
}

/// Count messages in a channel (for stats).
pub async fn count_channel_messages(
    pool: &PgPool,
    channel_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM messages WHERE channel_id = $1")
            .bind(channel_id)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}
