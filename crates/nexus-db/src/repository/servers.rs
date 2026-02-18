//! Server repository — CRUD operations for servers (guilds).

use nexus_common::models::server::{Invite, Server};
use sqlx::PgPool;
use uuid::Uuid;

/// Create a new server.
pub async fn create_server(
    pool: &PgPool,
    id: Uuid,
    name: &str,
    owner_id: Uuid,
    is_public: bool,
) -> Result<Server, sqlx::Error> {
    sqlx::query_as::<_, Server>(
        r#"
        INSERT INTO servers (id, name, owner_id, is_public, features, settings, member_count, created_at, updated_at)
        VALUES ($1, $2, $3, $4, '{}', '{}', 1, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(owner_id)
    .bind(is_public)
    .fetch_one(pool)
    .await
}

/// Find a server by ID.
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Server>, sqlx::Error> {
    sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// List servers a user is a member of.
pub async fn list_user_servers(pool: &PgPool, user_id: Uuid) -> Result<Vec<Server>, sqlx::Error> {
    sqlx::query_as::<_, Server>(
        r#"
        SELECT s.* FROM servers s
        INNER JOIN members m ON m.server_id = s.id
        WHERE m.user_id = $1
        ORDER BY s.name
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Update server details.
pub async fn update_server(
    pool: &PgPool,
    id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    is_public: Option<bool>,
) -> Result<Server, sqlx::Error> {
    sqlx::query_as::<_, Server>(
        r#"
        UPDATE servers SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            is_public = COALESCE($4, is_public),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(is_public)
    .fetch_one(pool)
    .await
}

/// Delete a server and all associated data.
pub async fn delete_server(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    // Cascading deletes handled by foreign keys
    sqlx::query("DELETE FROM servers WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Increment server member count.
pub async fn increment_member_count(pool: &PgPool, server_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE servers SET member_count = member_count + 1 WHERE id = $1")
        .bind(server_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Decrement server member count.
pub async fn decrement_member_count(pool: &PgPool, server_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE servers SET member_count = GREATEST(member_count - 1, 0) WHERE id = $1")
        .bind(server_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Create an invite link.
pub async fn create_invite(
    pool: &PgPool,
    code: &str,
    server_id: Uuid,
    channel_id: Option<Uuid>,
    inviter_id: Uuid,
    max_uses: Option<i32>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Invite, sqlx::Error> {
    sqlx::query_as::<_, Invite>(
        r#"
        INSERT INTO invites (code, server_id, channel_id, inviter_id, max_uses, uses, expires_at, created_at)
        VALUES ($1, $2, $3, $4, $5, 0, $6, NOW())
        RETURNING *
        "#,
    )
    .bind(code)
    .bind(server_id)
    .bind(channel_id)
    .bind(inviter_id)
    .bind(max_uses)
    .bind(expires_at)
    .fetch_one(pool)
    .await
}

/// Find an invite by code.
pub async fn find_invite(pool: &PgPool, code: &str) -> Result<Option<Invite>, sqlx::Error> {
    sqlx::query_as::<_, Invite>("SELECT * FROM invites WHERE code = $1")
        .bind(code)
        .fetch_optional(pool)
        .await
}

/// Consume an invite (increment use count).
pub async fn use_invite(pool: &PgPool, code: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE invites SET uses = uses + 1 WHERE code = $1")
        .bind(code)
        .execute(pool)
        .await?;
    Ok(())
}

/// List public/discoverable servers.
pub async fn list_public_servers(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Server>, sqlx::Error> {
    sqlx::query_as::<_, Server>(
        r#"
        SELECT * FROM servers
        WHERE is_public = true
        ORDER BY member_count DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}
