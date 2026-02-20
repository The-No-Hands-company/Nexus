//! Reactions repository — add/remove emoji reactions on messages.

use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

/// A reaction row from the database.
#[derive(Debug)]
pub struct ReactionRow {
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub emoji: String,
    pub created_at: DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> for ReactionRow {
    fn from_row(row: &'r sqlx::any::AnyRow) -> Result<Self, sqlx::Error> {
        use crate::any_compat::*;
        Ok(ReactionRow {
            message_id: get_uuid(row, "message_id")?,
            user_id: get_uuid(row, "user_id")?,
            emoji: row.try_get("emoji")?,
            created_at: get_datetime(row, "created_at")?,
        })
    }
}

/// Aggregated reaction count for a specific emoji on a message.
#[derive(Debug, sqlx::FromRow)]
pub struct ReactionCount {
    pub emoji: String,
    pub count: i64,
}

/// Add a reaction to a message. Returns true if newly added, false if already exists.
pub async fn add_reaction(
    pool: &sqlx::AnyPool,
    message_id: Uuid,
    user_id: Uuid,
    emoji: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO reactions (message_id, user_id, emoji, created_at)
        VALUES (?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT (message_id, user_id, emoji) DO NOTHING
        "#,
    )
    .bind(message_id.to_string())
    .bind(user_id.to_string())
    .bind(emoji)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Remove a reaction from a message.
pub async fn remove_reaction(
    pool: &sqlx::AnyPool,
    message_id: Uuid,
    user_id: Uuid,
    emoji: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM reactions WHERE message_id = ? AND user_id = ? AND emoji = ?",
    )
    .bind(message_id.to_string())
    .bind(user_id.to_string())
    .bind(emoji)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Remove all reactions of a specific emoji from a message (moderation).
pub async fn remove_all_reactions_for_emoji(
    pool: &sqlx::AnyPool,
    message_id: Uuid,
    emoji: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM reactions WHERE message_id = ? AND emoji = ?",
    )
    .bind(message_id.to_string())
    .bind(emoji)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Remove ALL reactions from a message.
pub async fn remove_all_reactions(
    pool: &sqlx::AnyPool,
    message_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM reactions WHERE message_id = ?")
        .bind(message_id.to_string())
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Get reaction counts for a message, grouped by emoji.
pub async fn get_reaction_counts(
    pool: &sqlx::AnyPool,
    message_id: Uuid,
) -> Result<Vec<ReactionCount>, sqlx::Error> {
    sqlx::query_as::<_, ReactionCount>(
        r#"
        SELECT emoji, COUNT(*) as count
        FROM reactions
        WHERE message_id = ?
        GROUP BY emoji
        ORDER BY MIN(created_at) ASC
        "#,
    )
    .bind(message_id.to_string())
    .fetch_all(pool)
    .await
}

/// Check if a specific user has reacted with a specific emoji.
pub async fn has_user_reacted(
    pool: &sqlx::AnyPool,
    message_id: Uuid,
    user_id: Uuid,
    emoji: &str,
) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM reactions WHERE message_id = ? AND user_id = ? AND emoji = ?) AS ex",
    )
    .bind(message_id.to_string())
    .bind(user_id.to_string())
    .bind(emoji)
    .fetch_one(pool)
    .await?;
    Ok(row.0 != 0)
}

/// Get users who reacted with a specific emoji on a message.
pub async fn get_reactors(
    pool: &sqlx::AnyPool,
    message_id: Uuid,
    emoji: &str,
    limit: i64,
) -> Result<Vec<Uuid>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT user_id FROM reactions
        WHERE message_id = ? AND emoji = ?
        ORDER BY created_at ASC
        LIMIT ?
        "#,
    )
    .bind(message_id.to_string())
    .bind(emoji)
    .bind(limit.min(100))
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.0.parse().ok())
        .collect())
}
