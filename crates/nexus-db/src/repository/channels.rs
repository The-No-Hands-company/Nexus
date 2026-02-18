//! Channel repository.

use nexus_common::models::channel::Channel;
use sqlx::PgPool;
use uuid::Uuid;

/// Create a new channel.
pub async fn create_channel(
    pool: &PgPool,
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
        VALUES ($1, $2, $3, $4::channel_type, $5, $6, $7, false, 0, false, '[]', false, false, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(server_id)
    .bind(parent_id)
    .bind(channel_type)
    .bind(name)
    .bind(topic)
    .bind(position)
    .fetch_one(pool)
    .await
}

/// List channels in a server.
pub async fn list_server_channels(
    pool: &PgPool,
    server_id: Uuid,
) -> Result<Vec<Channel>, sqlx::Error> {
    sqlx::query_as::<_, Channel>(
        "SELECT * FROM channels WHERE server_id = $1 ORDER BY position, created_at",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
}

/// Find a channel by ID.
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Channel>, sqlx::Error> {
    sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Update a channel.
pub async fn update_channel(
    pool: &PgPool,
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
            name = COALESCE($2, name),
            topic = COALESCE($3, topic),
            position = COALESCE($4, position),
            nsfw = COALESCE($5, nsfw),
            rate_limit_per_user = COALESCE($6, rate_limit_per_user),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(topic)
    .bind(position)
    .bind(nsfw)
    .bind(rate_limit_per_user)
    .fetch_one(pool)
    .await
}

/// Delete a channel.
pub async fn delete_channel(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM channels WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Create a DM channel between two users.
pub async fn find_or_create_dm(
    pool: &PgPool,
    id: Uuid,
    user1: Uuid,
    user2: Uuid,
) -> Result<Channel, sqlx::Error> {
    // Check for existing DM
    let existing = sqlx::query_as::<_, Channel>(
        r#"
        SELECT c.* FROM channels c
        INNER JOIN dm_participants dp1 ON dp1.channel_id = c.id AND dp1.user_id = $1
        INNER JOIN dm_participants dp2 ON dp2.channel_id = c.id AND dp2.user_id = $2
        WHERE c.channel_type = 'dm'
        LIMIT 1
        "#,
    )
    .bind(user1)
    .bind(user2)
    .fetch_optional(pool)
    .await?;

    if let Some(channel) = existing {
        return Ok(channel);
    }

    // Create new DM
    let channel = create_channel(pool, id, None, None, "dm", None, None, 0).await?;

    // Add participants
    sqlx::query("INSERT INTO dm_participants (channel_id, user_id) VALUES ($1, $2), ($1, $3)")
        .bind(channel.id)
        .bind(user1)
        .bind(user2)
        .execute(pool)
        .await?;

    Ok(channel)
}
