//! Server repository — CRUD operations for servers (guilds).

use nexus_common::models::server::{Invite, Server};

use uuid::Uuid;

/// Create a new server.
pub async fn create_server(
    pool: &sqlx::AnyPool,
    id: Uuid,
    name: &str,
    owner_id: Uuid,
    is_public: bool,
) -> Result<Server, sqlx::Error> {
    sqlx::query_as::<_, Server>(
        r#"
        INSERT INTO servers (id, name, owner_id, is_public, features, settings, member_count, created_at, updated_at)
        VALUES (?, ?, ?, ?, '{}', '{}', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(name)
    .bind(owner_id.to_string())
    .bind(is_public)
    .fetch_one(pool)
    .await
}

/// Find a server by ID.
pub async fn find_by_id(pool: &sqlx::AnyPool, id: Uuid) -> Result<Option<Server>, sqlx::Error> {
    sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

/// List servers a user is a member of.
pub async fn list_user_servers(pool: &sqlx::AnyPool, user_id: Uuid) -> Result<Vec<Server>, sqlx::Error> {
    sqlx::query_as::<_, Server>(
        r#"
        SELECT s.* FROM servers s
        INNER JOIN members m ON m.server_id = s.id
        WHERE m.user_id = ?
        ORDER BY s.name
        "#,
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await
}

/// Update server details.
pub async fn update_server(
    pool: &sqlx::AnyPool,
    id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    is_public: Option<bool>,
) -> Result<Server, sqlx::Error> {
    sqlx::query_as::<_, Server>(
        r#"
        UPDATE servers SET
            name = COALESCE(?, name),
            description = COALESCE(?, description),
            is_public = COALESCE(?, is_public),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(name)
    .bind(description)
    .bind(is_public)
    .fetch_one(pool)
    .await
}

/// Delete a server and all associated data.
pub async fn delete_server(pool: &sqlx::AnyPool, id: Uuid) -> Result<(), sqlx::Error> {
    // Cascading deletes handled by foreign keys
    sqlx::query("DELETE FROM servers WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Increment server member count.
pub async fn increment_member_count(pool: &sqlx::AnyPool, server_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE servers SET member_count = member_count + 1 WHERE id = ?")
        .bind(server_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Decrement server member count.
pub async fn decrement_member_count(pool: &sqlx::AnyPool, server_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE servers SET member_count = max(member_count - 1, 0) WHERE id = ?")
        .bind(server_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Create an invite link.
pub async fn create_invite(
    pool: &sqlx::AnyPool,
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
        VALUES (?, ?, ?, ?, ?, 0, ?, CURRENT_TIMESTAMP)
        RETURNING *
        "#,
    )
    .bind(code)
    .bind(server_id.to_string())
    .bind(channel_id.map(|u| u.to_string()))
    .bind(inviter_id.to_string())
    .bind(max_uses)
    .bind(expires_at.map(|x| x.to_rfc3339()))
    .fetch_one(pool)
    .await
}

/// Find an invite by code.
pub async fn find_invite(pool: &sqlx::AnyPool, code: &str) -> Result<Option<Invite>, sqlx::Error> {
    sqlx::query_as::<_, Invite>("SELECT * FROM invites WHERE code = ?")
        .bind(code)
        .fetch_optional(pool)
        .await
}

/// Consume an invite (increment use count).
pub async fn use_invite(pool: &sqlx::AnyPool, code: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE invites SET uses = uses + 1 WHERE code = ?")
        .bind(code)
        .execute(pool)
        .await?;
    Ok(())
}

/// List public/discoverable servers.
pub async fn list_public_servers(
    pool: &sqlx::AnyPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Server>, sqlx::Error> {
    sqlx::query_as::<_, Server>(
        r#"
        SELECT * FROM servers
        WHERE is_public = true
        ORDER BY member_count DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}
