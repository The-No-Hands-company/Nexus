//! Read state repository — tracks where each user has read up to per channel.

use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

/// Read state row.
#[derive(Debug)]
pub struct ReadStateRow {
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub last_read_message_id: Option<Uuid>,
    pub mention_count: i32,
    pub last_read_at: DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> for ReadStateRow {
    fn from_row(row: &'r sqlx::any::AnyRow) -> Result<Self, sqlx::Error> {
        use crate::any_compat::*;
        Ok(ReadStateRow {
            user_id: get_uuid(row, "user_id")?,
            channel_id: get_uuid(row, "channel_id")?,
            last_read_message_id: get_opt_uuid(row, "last_read_message_id")?,
            mention_count: row.try_get("mention_count")?,
            last_read_at: get_datetime(row, "last_read_at")?,
        })
    }
}

/// Acknowledge reading a channel up to a specific message.
/// Resets mention count to 0.
pub async fn ack_message(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    channel_id: Uuid,
    message_id: Uuid,
) -> Result<ReadStateRow, sqlx::Error> {
    sqlx::query_as::<_, ReadStateRow>(
        r#"
        INSERT INTO read_states (user_id, channel_id, last_read_message_id, mention_count, last_read_at)
        VALUES (?, ?, ?, 0, CURRENT_TIMESTAMP)
        ON CONFLICT (user_id, channel_id) DO UPDATE SET
            last_read_message_id = ?,
            mention_count = 0,
            last_read_at = CURRENT_TIMESTAMP
        RETURNING *
        "#,
    )
    .bind(user_id.to_string())
    .bind(channel_id.to_string())
    .bind(message_id.to_string())
    .bind(message_id.to_string())
    .fetch_one(pool)
    .await
}

/// Increment mention count for a user in a channel (called when a message mentions them).
pub async fn increment_mention_count(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    channel_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO read_states (user_id, channel_id, mention_count, last_read_at)
        VALUES (?, ?, 1, CURRENT_TIMESTAMP)
        ON CONFLICT (user_id, channel_id) DO UPDATE SET
            mention_count = read_states.mention_count + 1
        "#,
    )
    .bind(user_id.to_string())
    .bind(channel_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

/// Get a user's read state for a specific channel.
pub async fn get_read_state(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    channel_id: Uuid,
) -> Result<Option<ReadStateRow>, sqlx::Error> {
    sqlx::query_as::<_, ReadStateRow>(
        "SELECT * FROM read_states WHERE user_id = ? AND channel_id = ?",
    )
    .bind(user_id.to_string())
    .bind(channel_id.to_string())
    .fetch_optional(pool)
    .await
}

/// Get all read states for a user (for READY payload).
pub async fn get_all_read_states(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
) -> Result<Vec<ReadStateRow>, sqlx::Error> {
    sqlx::query_as::<_, ReadStateRow>(
        "SELECT * FROM read_states WHERE user_id = ?",
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await
}

/// Get unread channel IDs for a user (channels where last_read_message_id < channel.last_message_id).
pub async fn get_unread_channels(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
) -> Result<Vec<UnreadChannel>, sqlx::Error> {
    sqlx::query_as::<_, UnreadChannel>(
        r#"
        SELECT
            c.id as channel_id,
            c.last_message_id,
            rs.last_read_message_id,
            COALESCE(rs.mention_count, 0) as mention_count
        FROM channels c
        LEFT JOIN read_states rs ON rs.channel_id = c.id AND rs.user_id = ?
        WHERE (
            -- User is in a server that has this channel
            c.server_id IN (SELECT server_id FROM members WHERE user_id = ?)
            -- OR user is a DM participant
            OR c.id IN (SELECT channel_id FROM dm_participants WHERE user_id = ?)
        )
        AND c.last_message_id IS NOT NULL
        AND (rs.last_read_message_id IS NULL OR rs.last_read_message_id < c.last_message_id)
        "#,
    )
    .bind(user_id.to_string())
    .bind(user_id.to_string())
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await
}

/// Unread channel info.
#[derive(Debug)]
pub struct UnreadChannel {
    pub channel_id: Uuid,
    pub last_message_id: Option<Uuid>,
    pub last_read_message_id: Option<Uuid>,
    pub mention_count: i32,
}

impl<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> for UnreadChannel {
    fn from_row(row: &'r sqlx::any::AnyRow) -> Result<Self, sqlx::Error> {
        use crate::any_compat::*;
        Ok(UnreadChannel {
            channel_id: get_uuid(row, "channel_id")?,
            last_message_id: get_opt_uuid(row, "last_message_id")?,
            last_read_message_id: get_opt_uuid(row, "last_read_message_id")?,
            mention_count: row.try_get("mention_count")?,
        })
    }
}

/// Delete all read states for a user in a specific server's channels (on leave).
pub async fn delete_server_read_states(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM read_states
        WHERE user_id = ?
        AND channel_id IN (SELECT id FROM channels WHERE server_id = ?)
        "#,
    )
    .bind(user_id.to_string())
    .bind(server_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}
