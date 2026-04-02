//! User repository — CRUD operations for user accounts.

use nexus_common::models::user::User;
use std::collections::HashMap;

use uuid::Uuid;

use crate::select_cols::USER_COLS;

/// Create a new user account.
pub async fn create_user(
    pool: &sqlx::AnyPool,
    id: Uuid,
    username: &str,
    email: Option<&str>,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    let q = format!(
        "INSERT INTO users (id, username, email, password_hash, presence, flags, created_at, updated_at) \
         VALUES ($1::uuid, $2, $3, $4, 'offline', 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
         RETURNING {USER_COLS}"
    );
    sqlx::query_as::<_, User>(&q)
    .bind(id.to_string())
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await
}

/// Find a user by their unique ID.
pub async fn find_by_id(pool: &sqlx::AnyPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    let q = format!("SELECT {USER_COLS} FROM users WHERE id = $1::uuid");
    sqlx::query_as::<_, User>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

/// Find users by IDs and return a map keyed by user id.
pub async fn find_by_ids_map(
    pool: &sqlx::AnyPool,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, User>, sqlx::Error> {
    let mut out = HashMap::new();
    if ids.is_empty() {
        return Ok(out);
    }

    let mut qb = sqlx::QueryBuilder::new(format!("SELECT {USER_COLS} FROM users WHERE id IN ("));
    {
        let mut separated = qb.separated(", ");
        for id in ids {
            separated.push_bind(id.to_string());
        }
    }
    qb.push(")");

    let rows = qb.build_query_as::<User>().fetch_all(pool).await?;
    for user in rows {
        out.insert(user.id, user);
    }
    Ok(out)
}

/// Find a user by username (case-insensitive).
pub async fn find_by_username(pool: &sqlx::AnyPool, username: &str) -> Result<Option<User>, sqlx::Error> {
    let q = format!("SELECT {USER_COLS} FROM users WHERE LOWER(username) = LOWER($1)");
    sqlx::query_as::<_, User>(&q)
        .bind(username)
        .fetch_optional(pool)
        .await
}

/// Find a user by email.
pub async fn find_by_email(pool: &sqlx::AnyPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    let q = format!("SELECT {USER_COLS} FROM users WHERE LOWER(email) = LOWER($1)");
    sqlx::query_as::<_, User>(&q)
        .bind(email)
        .fetch_optional(pool)
        .await
}

/// Update user profile fields.
pub async fn update_user(
    pool: &sqlx::AnyPool,
    id: Uuid,
    username: Option<&str>,
    display_name: Option<&str>,
    bio: Option<&str>,
    status: Option<&str>,
    avatar: Option<&str>,
) -> Result<User, sqlx::Error> {
    let q = format!(
        "UPDATE users SET \
             username = COALESCE($1, username), \
             display_name = COALESCE($2, display_name), \
             bio = COALESCE($3, bio), \
             status = COALESCE($4, status), \
             avatar = COALESCE($6, avatar), \
             updated_at = CURRENT_TIMESTAMP \
           WHERE id = $5::uuid \
           RETURNING {USER_COLS}"
    );
    sqlx::query_as::<_, User>(&q)
    .bind(username)
    .bind(display_name)
    .bind(bio)
    .bind(status)
    .bind(id.to_string())
    .bind(avatar)
    .fetch_one(pool)
    .await
}

/// Update user presence state.
pub async fn update_presence(
    pool: &sqlx::AnyPool,
    id: Uuid,
    presence: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET presence = $1::user_presence WHERE id = $2::uuid")
        .bind(presence)
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Update the password hash for a user account.
pub async fn update_password_hash(
    pool: &sqlx::AnyPool,
    id: Uuid,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET password_hash = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2::uuid",
    )
    .bind(password_hash)
    .bind(id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete a user account (soft delete — sets DISABLED flag).
pub async fn soft_delete_user(pool: &sqlx::AnyPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE users SET
            flags = flags | (1 << 5),
            email = NULL,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $1::uuid
        "#,
    )
    .bind(id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

/// Count total users (for admin dashboard).
pub async fn count_users(pool: &sqlx::AnyPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE (flags & (1 << 5)) = 0")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Execute due scheduled account deletions.
///
/// This applies a soft-delete policy:
/// - set DISABLED flag
/// - clear email
/// - clear password_hash
/// - clear scheduled_deletion_at
///
/// Returns number of affected users.
pub async fn purge_scheduled_deletions(pool: &sqlx::AnyPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE users SET \
            flags = flags | (1 << 5), \
            email = NULL, \
            password_hash = '', \
            scheduled_deletion_at = NULL, \
            updated_at = CURRENT_TIMESTAMP \
         WHERE scheduled_deletion_at IS NOT NULL \
           AND scheduled_deletion_at <= NOW() \
           AND (flags & (1 << 5)) = 0",
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

// ── Federation helpers ────────────────────────────────────────────────────────

/// Upsert a shadow user record for a remote federated user.
///
/// The remote user's UUID (from their home server) is used as the primary key —
/// UUIDs are globally unique, so collisions are practically impossible.  On
/// conflict the display name and avatar are refreshed.
pub async fn upsert_remote_user(
    pool: &sqlx::AnyPool,
    id: Uuid,
    username: &str,
    server_name: &str,
    display_name: Option<&str>,
    avatar: Option<&str>,
) -> Result<User, sqlx::Error> {
    let q = format!(
        "INSERT INTO users \
             (id, username, server_name, is_remote, display_name, avatar, \
              password_hash, presence, flags, created_at, updated_at) \
         VALUES ($1::uuid, $2, $3, TRUE, $4, $5, '', 'offline', 0, \
                 CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
         ON CONFLICT (id) DO UPDATE SET \
             display_name = EXCLUDED.display_name, \
             avatar       = EXCLUDED.avatar, \
             updated_at   = CURRENT_TIMESTAMP \
         RETURNING {USER_COLS}"
    );
    sqlx::query_as::<_, User>(&q)
        .bind(id.to_string())
        .bind(username)
        .bind(server_name)
        .bind(display_name)
        .bind(avatar)
        .fetch_one(pool)
        .await
}

/// Find a remote shadow user by (username, server_name).
pub async fn find_remote_by_username_and_server(
    pool: &sqlx::AnyPool,
    username: &str,
    server_name: &str,
) -> Result<Option<User>, sqlx::Error> {
    let q = format!(
        "SELECT {USER_COLS} FROM users \
         WHERE LOWER(username) = LOWER($1) AND server_name = $2 AND is_remote = TRUE"
    );
    sqlx::query_as::<_, User>(&q)
        .bind(username)
        .bind(server_name)
        .fetch_optional(pool)
        .await
}
