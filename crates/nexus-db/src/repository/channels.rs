//! Channel repository.

use nexus_common::models::channel::Channel;

use uuid::Uuid;

/// Create a new channel.
pub async fn create_channel(
    pool: &sqlx::AnyPool,
    id: Uuid,
    server_id: Option<Uuid>,
    parent_id: Option<Uuid>,
    channel_type: &str,
    name: Option<&str>,
    topic: Option<&str>,
    position: i32,
) -> Result<Channel, sqlx::Error> {
    sqlx::query_as::<_, Channel>(
        r#"
        INSERT INTO channels (
            id, server_id, parent_id, channel_type, name, topic, position,
            nsfw, rate_limit_per_user, encrypted, permission_overwrites,
            archived, locked, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, false, 0, false, '[]', false, false, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(server_id.map(|u| u.to_string()))
    .bind(parent_id.map(|u| u.to_string()))
    .bind(channel_type)
    .bind(name)
    .bind(topic)
    .bind(position)
    .fetch_one(pool)
    .await
}

/// List channels in a server.
pub async fn list_server_channels(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
) -> Result<Vec<Channel>, sqlx::Error> {
    sqlx::query_as::<_, Channel>(
        "SELECT * FROM channels WHERE server_id = ? ORDER BY position, created_at",
    )
    .bind(server_id.to_string())
    .fetch_all(pool)
    .await
}

/// Find a channel by ID.
pub async fn find_by_id(pool: &sqlx::AnyPool, id: Uuid) -> Result<Option<Channel>, sqlx::Error> {
    sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

/// Update a channel.
pub async fn update_channel(
    pool: &sqlx::AnyPool,
    id: Uuid,
    name: Option<&str>,
    topic: Option<&str>,
    position: Option<i32>,
    nsfw: Option<bool>,
    rate_limit_per_user: Option<i32>,
) -> Result<Channel, sqlx::Error> {
    sqlx::query_as::<_, Channel>(
        r#"
        UPDATE channels SET
            name = COALESCE(?, name),
            topic = COALESCE(?, topic),
            position = COALESCE(?, position),
            nsfw = COALESCE(?, nsfw),
            rate_limit_per_user = COALESCE(?, rate_limit_per_user),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(name)
    .bind(topic)
    .bind(position)
    .bind(nsfw)
    .bind(rate_limit_per_user)
    .fetch_one(pool)
    .await
}

/// Delete a channel.
pub async fn delete_channel(pool: &sqlx::AnyPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM channels WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Create a DM channel between two users.
pub async fn find_or_create_dm(
    pool: &sqlx::AnyPool,
    id: Uuid,
    user1: Uuid,
    user2: Uuid,
) -> Result<Channel, sqlx::Error> {
    // Check for existing DM
    let existing = sqlx::query_as::<_, Channel>(
        r#"
        SELECT c.* FROM channels c
        INNER JOIN dm_participants dp1 ON dp1.channel_id = c.id AND dp1.user_id = ?
        INNER JOIN dm_participants dp2 ON dp2.channel_id = c.id AND dp2.user_id = ?
        WHERE c.channel_type = 'dm'
        LIMIT 1
        "#,
    )
    .bind(user1.to_string())
    .bind(user2.to_string())
    .fetch_optional(pool)
    .await?;

    if let Some(channel) = existing {
        return Ok(channel);
    }

    // Create new DM
    let channel = create_channel(pool, id, None, None, "dm", None, None, 0).await?;

    // Add participants
    sqlx::query("INSERT INTO dm_participants (channel_id, user_id) VALUES (?, ?), (?, ?)")
        .bind(channel.id.to_string())
        .bind(user1.to_string())
        .bind(channel.id.to_string())
        .bind(user2.to_string())
        .execute(pool)
        .await?;

    Ok(channel)
}
