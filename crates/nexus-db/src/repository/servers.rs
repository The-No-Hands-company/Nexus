//! Server repository — CRUD operations for servers (guilds).

use nexus_common::models::server::{Invite, Server};

use uuid::Uuid;

use crate::select_cols::{INVITE_COLS, SERVER_COLS, SERVER_COLS_S};

/// Create a new server.
pub async fn create_server(
    pool: &sqlx::AnyPool,
    id: Uuid,
    name: &str,
    owner_id: Uuid,
    is_public: bool,
) -> Result<Server, sqlx::Error> {
    let q = format!(
        "INSERT INTO servers (id, name, owner_id, is_public, features, settings, member_count, created_at, updated_at) \
         VALUES ($1::uuid, $2, $3::uuid, $4, '{{}}', '{{}}', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
         RETURNING {SERVER_COLS}"
    );
    sqlx::query_as::<_, Server>(&q)
    .bind(id.to_string())
    .bind(name)
    .bind(owner_id.to_string())
    .bind(is_public)
    .fetch_one(pool)
    .await
}

/// Find a server by ID.
pub async fn find_by_id(pool: &sqlx::AnyPool, id: Uuid) -> Result<Option<Server>, sqlx::Error> {
    let q = format!("SELECT {SERVER_COLS} FROM servers WHERE id = $1::uuid");
    sqlx::query_as::<_, Server>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

/// List servers a user is a member of.
pub async fn list_user_servers(pool: &sqlx::AnyPool, user_id: Uuid) -> Result<Vec<Server>, sqlx::Error> {
    let q = format!(
        "SELECT {SERVER_COLS_S} FROM servers s \
         INNER JOIN members m ON m.server_id = s.id \
         WHERE m.user_id = $1::uuid \
         ORDER BY s.name"
    );
    sqlx::query_as::<_, Server>(&q)
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
    let q = format!(
        "UPDATE servers SET \
             name = COALESCE($1, name), \
             description = COALESCE($2, description), \
             is_public = COALESCE($3, is_public), \
             updated_at = CURRENT_TIMESTAMP \
         WHERE id = $4::uuid \
         RETURNING {SERVER_COLS}"
    );
    sqlx::query_as::<_, Server>(&q)
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
    sqlx::query("DELETE FROM servers WHERE id = $1::uuid")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Increment server member count.
pub async fn increment_member_count(pool: &sqlx::AnyPool, server_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE servers SET member_count = member_count + 1 WHERE id = $1::uuid")
        .bind(server_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Decrement server member count.
pub async fn decrement_member_count(pool: &sqlx::AnyPool, server_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE servers SET member_count = max(member_count - 1, 0) WHERE id = $1::uuid")
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
    let q = format!(
        "INSERT INTO invites (code, server_id, channel_id, inviter_id, max_uses, uses, expires_at, created_at) \
         VALUES ($1, $2::uuid, $3::uuid, $4::uuid, $5, 0, $6::timestamptz, CURRENT_TIMESTAMP) \
         RETURNING {INVITE_COLS}"
    );
    sqlx::query_as::<_, Invite>(&q)
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
    let q = format!("SELECT {INVITE_COLS} FROM invites WHERE code = $1");
    sqlx::query_as::<_, Invite>(&q)
        .bind(code)
        .fetch_optional(pool)
        .await
}

/// Consume an invite (increment use count).
pub async fn use_invite(pool: &sqlx::AnyPool, code: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE invites SET uses = uses + 1 WHERE code = $1")
        .bind(code)
        .execute(pool)
        .await?;
    Ok(())
}

/// List all active (non-expired, non-exhausted) invites for a server.
pub async fn list_server_invites(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
) -> Result<Vec<Invite>, sqlx::Error> {
    let q = format!(
        "SELECT {INVITE_COLS} FROM invites \
         WHERE server_id = $1 \
           AND (expires_at IS NULL OR expires_at > NOW()) \
           AND (max_uses IS NULL OR uses < max_uses) \
         ORDER BY created_at DESC \
         LIMIT 50"
    );
    sqlx::query_as::<_, Invite>(&q)
        .bind(server_id.to_string())
        .fetch_all(pool)
        .await
}

/// List public/discoverable servers.
pub async fn list_public_servers(
    pool: &sqlx::AnyPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Server>, sqlx::Error> {
    let q = format!(
        "SELECT {SERVER_COLS} FROM servers \
         WHERE is_public = true \
         ORDER BY member_count DESC \
         LIMIT $1 OFFSET $2"
    );
    sqlx::query_as::<_, Server>(&q)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}
