//! Matrix Application Service bridge persistence.
//!
//! Manages:
//! - `matrix_bridge_rooms`: channel ↔ Matrix room mappings
//! - `matrix_ghost_users`: synthetic Nexus accounts for Matrix participants

use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;

// ─── Types ───────────────────────────────────────────────────────────────────

/// A persisted channel ↔ Matrix room mapping.
#[derive(Debug, Clone)]
pub struct MatrixBridgeRoom {
    pub channel_id: Uuid,
    pub matrix_room_id: String,
    pub homeserver_url: String,
    pub as_token: String,
}

/// A ghost user representing a Matrix participant inside Nexus.
#[derive(Debug, Clone)]
pub struct MatrixGhostUser {
    pub id: Uuid,
    pub mxid: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

// ─── Room mappings ────────────────────────────────────────────────────────────

/// Insert or replace a channel ↔ room mapping.
pub async fn upsert_bridge_room(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
    matrix_room_id: &str,
    homeserver_url: &str,
    as_token: &str,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO matrix_bridge_rooms (channel_id, matrix_room_id, homeserver_url, as_token)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (channel_id) DO UPDATE
               SET matrix_room_id = EXCLUDED.matrix_room_id,
                   homeserver_url  = EXCLUDED.homeserver_url,
                   as_token        = EXCLUDED.as_token,
                   updated_at      = CURRENT_TIMESTAMP"#,
    )
    .bind(channel_id.to_string())
    .bind(matrix_room_id)
    .bind(homeserver_url)
    .bind(as_token)
    .execute(pool)
    .await?;
    Ok(())
}

/// Look up the Matrix room ID for a Nexus channel.
pub async fn get_room_for_channel(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
) -> Result<Option<MatrixBridgeRoom>> {
    let row = sqlx::query(
        "SELECT channel_id::text, matrix_room_id, homeserver_url, as_token \
         FROM matrix_bridge_rooms WHERE channel_id = $1",
    )
    .bind(channel_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_bridge_room))
}

/// Look up the Nexus channel for a given Matrix room ID.
pub async fn get_channel_for_room(
    pool: &sqlx::AnyPool,
    matrix_room_id: &str,
) -> Result<Option<MatrixBridgeRoom>> {
    let row = sqlx::query(
        "SELECT channel_id::text, matrix_room_id, homeserver_url, as_token \
         FROM matrix_bridge_rooms WHERE matrix_room_id = $1",
    )
    .bind(matrix_room_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_bridge_room))
}

/// Delete a room mapping.
pub async fn delete_bridge_room(pool: &sqlx::AnyPool, channel_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM matrix_bridge_rooms WHERE channel_id = $1")
        .bind(channel_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// List all bridge room mappings (admin/diagnostic use).
pub async fn list_bridge_rooms(pool: &sqlx::AnyPool) -> Result<Vec<MatrixBridgeRoom>> {
    let rows = sqlx::query(
        "SELECT channel_id::text, matrix_room_id, homeserver_url, as_token \
         FROM matrix_bridge_rooms ORDER BY channel_id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_bridge_room).collect())
}

fn row_to_bridge_room(row: &sqlx::any::AnyRow) -> MatrixBridgeRoom {
    let channel_id_str: String = row.try_get("channel_id").unwrap_or_default();
    MatrixBridgeRoom {
        channel_id: channel_id_str.parse().unwrap_or_default(),
        matrix_room_id: row.try_get("matrix_room_id").unwrap_or_default(),
        homeserver_url: row.try_get("homeserver_url").unwrap_or_default(),
        as_token: row.try_get("as_token").unwrap_or_default(),
    }
}

// ─── Ghost users ──────────────────────────────────────────────────────────────

/// Find or create a ghost Nexus user for a Matrix participant.
///
/// Returns the ghost's Nexus UUID so it can be used as `author_id` on
/// bridged messages.
pub async fn find_or_create_ghost(
    pool: &sqlx::AnyPool,
    mxid: &str,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<MatrixGhostUser> {
    // Check if ghost already exists.
    if let Some(ghost) = get_ghost_by_mxid(pool, mxid).await? {
        // Update display name / avatar if provided and changed.
        if display_name.is_some() || avatar_url.is_some() {
            sqlx::query(
                "UPDATE matrix_ghost_users \
                 SET display_name = COALESCE($1, display_name), \
                     avatar_url   = COALESCE($2, avatar_url), \
                     updated_at   = CURRENT_TIMESTAMP \
                 WHERE mxid = $3",
            )
            .bind(display_name)
            .bind(avatar_url)
            .bind(mxid)
            .execute(pool)
            .await?;
        }
        return Ok(ghost);
    }

    // Derive a stable username from the MXID: @user:server → matrix_user_server
    let username = mxid_to_username(mxid);
    let ghost_id = Uuid::new_v4();

    let row = sqlx::query(
        r#"INSERT INTO matrix_ghost_users (id, mxid, username, display_name, avatar_url)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (mxid) DO UPDATE
               SET display_name = COALESCE(EXCLUDED.display_name, matrix_ghost_users.display_name),
                   avatar_url   = COALESCE(EXCLUDED.avatar_url,   matrix_ghost_users.avatar_url),
                   updated_at   = CURRENT_TIMESTAMP
           RETURNING id::text, mxid, username, display_name, avatar_url"#,
    )
    .bind(ghost_id.to_string())
    .bind(mxid)
    .bind(&username)
    .bind(display_name)
    .bind(avatar_url)
    .fetch_one(pool)
    .await?;

    Ok(row_to_ghost(&row))
}

/// Look up a ghost user by MXID.
pub async fn get_ghost_by_mxid(
    pool: &sqlx::AnyPool,
    mxid: &str,
) -> Result<Option<MatrixGhostUser>> {
    let row = sqlx::query(
        "SELECT id::text, mxid, username, display_name, avatar_url \
         FROM matrix_ghost_users WHERE mxid = $1",
    )
    .bind(mxid)
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(row_to_ghost))
}

/// Look up a ghost user by their synthetic Nexus username.
pub async fn get_ghost_by_username(
    pool: &sqlx::AnyPool,
    username: &str,
) -> Result<Option<MatrixGhostUser>> {
    let row = sqlx::query(
        "SELECT id::text, mxid, username, display_name, avatar_url \
         FROM matrix_ghost_users WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(row_to_ghost))
}

fn row_to_ghost(row: &sqlx::any::AnyRow) -> MatrixGhostUser {
    let id_str: String = row.try_get("id").unwrap_or_default();
    MatrixGhostUser {
        id: id_str.parse().unwrap_or_default(),
        mxid: row.try_get("mxid").unwrap_or_default(),
        username: row.try_get("username").unwrap_or_default(),
        display_name: row.try_get("display_name").unwrap_or(None),
        avatar_url: row.try_get("avatar_url").unwrap_or(None),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Convert a Matrix ID to a Nexus-safe ghost username.
///
/// `@alice:matrix.org` → `matrix_alice_matrix.org`
/// `@bot:server.tld`   → `matrix_bot_server.tld`
pub fn mxid_to_username(mxid: &str) -> String {
    // Strip leading '@', replace ':' with '_', prepend 'matrix_'
    let stripped = mxid.trim_start_matches('@');
    let sanitised = stripped
        .replace(':', "_")
        .replace([' ', '#', '!', '$'], "_");
    format!("matrix_{}", &sanitised[..sanitised.len().min(48)])
}
