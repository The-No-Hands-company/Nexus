//! Thread repository — CRUD and membership for message threads.
//!
//! Threads are backed by a `channels` record plus a row in the `threads` table.
//! This repo handles only the `threads` + `thread_members` tables.

use nexus_common::models::rich::ThreadRow;
use uuid::Uuid;

// Module-level helper for member list query — uses String to avoid AnyPool Uuid decode issues
#[derive(sqlx::FromRow)]
struct ThreadMemberRow {
    user_id: String,
}

// ============================================================
// Create
// ============================================================

/// Create a thread record. Assumes the corresponding channel row already exists.
#[allow(clippy::too_many_arguments)]
pub async fn create_thread(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
    parent_channel_id: Uuid,
    parent_message_id: Option<Uuid>,
    owner_id: Uuid,
    title: &str,
    auto_archive_minutes: i32,
    tags: &[String],
) -> Result<ThreadRow, sqlx::Error> {
    let tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string());
    sqlx::query_as::<_, ThreadRow>(
        r#"
        INSERT INTO threads (
            channel_id, parent_message_id, owner_id, title,
            message_count, member_count, auto_archive_minutes,
            archived, locked, tags,
            created_at, updated_at
        )
        VALUES (?, ?, ?, ?, 0, 1, ?, false, false, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        RETURNING *, ? AS parent_channel_id
        "#,
    )
    .bind(channel_id.to_string())
    .bind(parent_message_id.map(|x| x.to_string()))
    .bind(owner_id.to_string())
    .bind(title)
    .bind(auto_archive_minutes)
    .bind(&tags_json)
    .bind(parent_channel_id.to_string())
    .fetch_one(pool)
    .await
}

// ============================================================
// Read
// ============================================================

/// Get a thread by its channel ID, including parent_channel_id.
pub async fn find_by_id(pool: &sqlx::AnyPool, channel_id: Uuid) -> Result<Option<ThreadRow>, sqlx::Error> {
    sqlx::query_as::<_, ThreadRow>(
        r#"
        SELECT t.*, c.parent_id AS parent_channel_id
        FROM threads t
        JOIN channels c ON c.id = t.channel_id
        WHERE t.channel_id = ?
        "#,
    )
    .bind(channel_id.to_string())
    .fetch_optional(pool)
    .await
}

/// List active (non-archived) threads in a channel.
pub async fn list_active(
    pool: &sqlx::AnyPool,
    parent_channel_id: Uuid,
    limit: i64,
) -> Result<Vec<ThreadRow>, sqlx::Error> {
    sqlx::query_as::<_, ThreadRow>(
        r#"
        SELECT t.*, c.parent_id AS parent_channel_id
        FROM threads t
        JOIN channels c ON c.id = t.channel_id
        WHERE c.parent_id = ?
          AND t.archived = false
          AND t.locked = false
        ORDER BY t.updated_at DESC
        LIMIT ?
        "#,
    )
    .bind(parent_channel_id.to_string())
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// List archived threads in a channel.
pub async fn list_archived(
    pool: &sqlx::AnyPool,
    parent_channel_id: Uuid,
    limit: i64,
    before: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Vec<ThreadRow>, sqlx::Error> {
    if let Some(b) = before {
        sqlx::query_as::<_, ThreadRow>(
            r#"
            SELECT t.*, c.parent_id AS parent_channel_id
            FROM threads t
            JOIN channels c ON c.id = t.channel_id
            WHERE c.parent_id = ?
              AND t.archived = true
              AND t.archived_at < ?
            ORDER BY t.archived_at DESC
            LIMIT ?
            "#,
        )
        .bind(parent_channel_id.to_string())
        .bind(b.to_rfc3339())
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, ThreadRow>(
            r#"
            SELECT t.*, c.parent_id AS parent_channel_id
            FROM threads t
            JOIN channels c ON c.id = t.channel_id
            WHERE c.parent_id = ?
              AND t.archived = true
            ORDER BY t.archived_at DESC
            LIMIT ?
            "#,
        )
        .bind(parent_channel_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

// ============================================================
// Update
// ============================================================

/// Update thread metadata.
pub async fn update_thread(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
    title: Option<&str>,
    archived: Option<bool>,
    locked: Option<bool>,
    auto_archive_minutes: Option<i32>,
    tags: Option<&[String]>,
) -> Result<ThreadRow, sqlx::Error> {
    let archived_at_clause = if archived == Some(true) {
        "archived_at = CURRENT_TIMESTAMP,"
    } else if archived == Some(false) {
        "archived_at = NULL,"
    } else {
        ""
    };
    let tags_json: Option<String> = tags.map(|t| serde_json::to_string(t).unwrap_or_else(|_| "[]".to_string()));

    let row = sqlx::query_as::<_, ThreadRow>(
        &format!(
            r#"
            UPDATE threads
            SET
                title = COALESCE(?, title),
                archived = COALESCE(?, archived),
                {archived_at_clause}
                locked = COALESCE(?, locked),
                auto_archive_minutes = COALESCE(?, auto_archive_minutes),
                tags = COALESCE(?, tags),
                updated_at = CURRENT_TIMESTAMP
            WHERE channel_id = ?
            RETURNING *, (SELECT parent_id FROM channels WHERE id = ?) AS parent_channel_id
            "#
        ),
    )
    .bind(title)
    .bind(archived)
    .bind(locked)
    .bind(auto_archive_minutes)
    .bind(tags_json)
    .bind(channel_id.to_string())
    .bind(channel_id.to_string())
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Increment the thread message count.
pub async fn increment_message_count(pool: &sqlx::AnyPool, channel_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE threads SET message_count = message_count + 1, updated_at = CURRENT_TIMESTAMP WHERE channel_id = ?",
    )
    .bind(channel_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

// ============================================================
// Thread membership
// ============================================================

/// Add a user as a thread member.
pub async fn add_member(pool: &sqlx::AnyPool, thread_id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO thread_members (thread_id, user_id, joined_at)
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT (thread_id, user_id) DO NOTHING
        "#,
    )
    .bind(thread_id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await?;

    // Recount members
    sqlx::query(
        r#"
        UPDATE threads
        SET member_count = (SELECT COUNT(*) FROM thread_members WHERE thread_id = ?),
            updated_at = CURRENT_TIMESTAMP
        WHERE channel_id = ?
        "#,
    )
    .bind(thread_id.to_string())
    .bind(thread_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

/// Remove a user from a thread.
pub async fn remove_member(
    pool: &sqlx::AnyPool,
    thread_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM thread_members WHERE thread_id = ? AND user_id = ?",
    )
    .bind(thread_id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        sqlx::query(
            r#"
            UPDATE threads
            SET member_count = (SELECT COUNT(*) FROM thread_members WHERE thread_id = ?),
                updated_at = CURRENT_TIMESTAMP
            WHERE channel_id = ?
            "#,
        )
        .bind(thread_id.to_string())
        .bind(thread_id.to_string())
        .execute(pool)
        .await?;
    }

    Ok(result.rows_affected() > 0)
}

/// Check if a user is a member of a thread.
pub async fn is_member(pool: &sqlx::AnyPool, thread_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let row = sqlx::query(
        "SELECT 1 FROM thread_members WHERE thread_id = ? AND user_id = ?",
    )
    .bind(thread_id.to_string())
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

/// List all members of a thread.
pub async fn list_members(
    pool: &sqlx::AnyPool,
    thread_id: Uuid,
) -> Result<Vec<uuid::Uuid>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ThreadMemberRow>(
        "SELECT user_id FROM thread_members WHERE thread_id = ? ORDER BY joined_at",
    )
    .bind(thread_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.user_id.parse().ok())
        .collect())
}
