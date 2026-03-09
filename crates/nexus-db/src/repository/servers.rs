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
    tags: &[String],
    category: Option<&str>,
) -> Result<Server, sqlx::Error> {
    let tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".into());
    let q = format!(
        "INSERT INTO servers (id, name, owner_id, is_public, features, settings, member_count, \
         tags, category, created_at, updated_at) \
         VALUES ($1::uuid, $2, $3::uuid, $4, '{{}}', '{{}}', 1, \
         ARRAY(SELECT jsonb_array_elements_text($5::jsonb)), $6, \
         CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
         RETURNING {SERVER_COLS}"
    );
    sqlx::query_as::<_, Server>(&q)
    .bind(id.to_string())
    .bind(name)
    .bind(owner_id.to_string())
    .bind(is_public)
    .bind(&tags_json)
    .bind(category)
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
    require_2fa: Option<bool>,
    spam_window_secs: Option<i32>,
    spam_max_messages: Option<i32>,
    tags: Option<&[String]>,
    category: Option<&str>,
    tip_jar_url: Option<&str>,
) -> Result<Server, sqlx::Error> {
    // Build tags update expression: only touch the column when tags is Some
    let tags_json = tags.map(|t| serde_json::to_string(t).unwrap_or_else(|_| "[]".into()));
    let q = format!(
        "UPDATE servers SET \
             name = COALESCE($1, name), \
             description = COALESCE($2, description), \
             is_public = COALESCE($3, is_public), \
             require_2fa = COALESCE($4, require_2fa), \
             spam_window_secs = COALESCE($5, spam_window_secs), \
             spam_max_messages = COALESCE($6, spam_max_messages), \
             tags = CASE WHEN $8::text IS NOT NULL THEN ARRAY(SELECT jsonb_array_elements_text($8::jsonb)) ELSE tags END, \
             category = COALESCE($9, category), \
             tip_jar_url = COALESCE($10, tip_jar_url), \
             updated_at = CURRENT_TIMESTAMP \
         WHERE id = $7::uuid \
         RETURNING {SERVER_COLS}"
    );
    sqlx::query_as::<_, Server>(&q)
    .bind(name)
    .bind(description)
    .bind(is_public)
    .bind(require_2fa)
    .bind(spam_window_secs)
    .bind(spam_max_messages)
    .bind(id.to_string())
    .bind(tags_json.as_deref())
    .bind(category)
    .bind(tip_jar_url)
    .fetch_one(pool)
    .await
}

/// Transfer server ownership to another member.
///
/// The caller is responsible for verifying the requester is the current owner
/// and that `new_owner_id` is already a member of the server.
pub async fn transfer_ownership(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
    new_owner_id: Uuid,
) -> Result<Server, sqlx::Error> {
    let q = format!(
        "UPDATE servers SET \
             owner_id = $1::uuid, \
             updated_at = CURRENT_TIMESTAMP \
         WHERE id = $2::uuid \
         RETURNING {SERVER_COLS}"
    );
    sqlx::query_as::<_, Server>(&q)
    .bind(new_owner_id.to_string())
    .bind(server_id.to_string())
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

// ── Discovery queries (v0.16) ─────────────────────────────────────────────────

/// List featured public servers (ordered by featured_at descending).
pub async fn list_featured_servers(
    pool: &sqlx::AnyPool,
    limit: i64,
) -> Result<Vec<Server>, sqlx::Error> {
    let q = format!(
        "SELECT {SERVER_COLS} FROM servers \
         WHERE is_public = true AND featured_at IS NOT NULL \
         ORDER BY featured_at DESC \
         LIMIT $1"
    );
    sqlx::query_as::<_, Server>(&q)
        .bind(limit)
        .fetch_all(pool)
        .await
}

/// List public servers filtered by category.
pub async fn list_servers_by_category(
    pool: &sqlx::AnyPool,
    category: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Server>, sqlx::Error> {
    let q = format!(
        "SELECT {SERVER_COLS} FROM servers \
         WHERE is_public = true AND category = $1 \
         ORDER BY activity_score DESC, member_count DESC \
         LIMIT $2 OFFSET $3"
    );
    sqlx::query_as::<_, Server>(&q)
        .bind(category)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

/// List public servers that contain a specific tag.
pub async fn list_servers_by_tag(
    pool: &sqlx::AnyPool,
    tag: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Server>, sqlx::Error> {
    let q = format!(
        "SELECT {SERVER_COLS} FROM servers \
         WHERE is_public = true AND $1 = ANY(tags) \
         ORDER BY activity_score DESC, member_count DESC \
         LIMIT $2 OFFSET $3"
    );
    sqlx::query_as::<_, Server>(&q)
        .bind(tag)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

/// Full-text search across public server names and descriptions.
pub async fn search_public_servers(
    pool: &sqlx::AnyPool,
    query: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Server>, sqlx::Error> {
    let q = format!(
        "SELECT {SERVER_COLS} FROM servers \
         WHERE is_public = true \
           AND to_tsvector('english', COALESCE(name, '') || ' ' || COALESCE(description, '')) \
               @@ plainto_tsquery('english', $1) \
         ORDER BY activity_score DESC, member_count DESC \
         LIMIT $2 OFFSET $3"
    );
    sqlx::query_as::<_, Server>(&q)
        .bind(query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

/// Get a server preview (public info + public channel list) without auth.
/// Returns `None` if the server doesn't exist or is not public.
pub async fn get_server_preview(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
) -> Result<Option<Server>, sqlx::Error> {
    let q = format!(
        "SELECT {SERVER_COLS} FROM servers \
         WHERE id = $1::uuid AND is_public = true"
    );
    sqlx::query_as::<_, Server>(&q)
        .bind(server_id.to_string())
        .fetch_optional(pool)
        .await
}

/// Feature or un-feature a server (admin action).
pub async fn set_featured(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
    featured: bool,
) -> Result<Server, sqlx::Error> {
    let q = format!(
        "UPDATE servers SET \
             featured_at = CASE WHEN $1 THEN CURRENT_TIMESTAMP ELSE NULL END, \
             updated_at = CURRENT_TIMESTAMP \
         WHERE id = $2::uuid \
         RETURNING {SERVER_COLS}"
    );
    sqlx::query_as::<_, Server>(&q)
        .bind(featured)
        .bind(server_id.to_string())
        .fetch_one(pool)
        .await
}

/// Update the activity_score for a server (called periodically by a background task).
pub async fn update_activity_score(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
    score: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE servers SET activity_score = $1 WHERE id = $2::uuid"
    )
        .bind(score)
        .bind(server_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}
