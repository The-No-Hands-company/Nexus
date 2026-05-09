//! User repository — CRUD operations for user accounts.

use nexus_common::models::user::User;
use std::collections::HashMap;

use uuid::Uuid;

use crate::select_cols::USER_COLS;

/// Create a new user account.
///
/// If `NEXUS__SECURITY__EMAIL_KEY` is set the email is encrypted with
/// AES-256-GCM before storage and an HMAC-SHA256 digest is written to
/// `email_hash` for index-based lookups.
pub async fn create_user(
    pool: &sqlx::AnyPool,
    id: Uuid,
    username: &str,
    email: Option<&str>,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    let stored_email = email.map(|e| crate::email_crypto::encrypt(e));
    let email_hash = email.and_then(|e| crate::email_crypto::lookup_hash(e));

    let q = format!(
        "INSERT INTO users \
           (id, username, email, email_hash, password_hash, presence, flags, created_at, updated_at) \
         VALUES ($1::uuid, $2, $3, $4, $5, 'offline', 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
         RETURNING {USER_COLS}"
    );
    sqlx::query_as::<_, User>(&q)
        .bind(id.to_string())
        .bind(username)
        .bind(stored_email.as_deref())
        .bind(email_hash.as_deref())
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
pub async fn find_by_username(
    pool: &sqlx::AnyPool,
    username: &str,
) -> Result<Option<User>, sqlx::Error> {
    let q = format!("SELECT {USER_COLS} FROM users WHERE LOWER(username) = LOWER($1)");
    sqlx::query_as::<_, User>(&q)
        .bind(username)
        .fetch_optional(pool)
        .await
}

/// Find a user by email.
///
/// When at-rest encryption is enabled (key configured) the lookup uses the
/// `email_hash` index.  Otherwise falls back to the plaintext `email` column
/// for backward compatibility with unencrypted rows.
pub async fn find_by_email(pool: &sqlx::AnyPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    if let Some(hash) = crate::email_crypto::lookup_hash(email) {
        // Encrypted path: fast index lookup on the HMAC digest.
        let q = format!("SELECT {USER_COLS} FROM users WHERE email_hash = $1");
        sqlx::query_as::<_, User>(&q)
            .bind(&hash)
            .fetch_optional(pool)
            .await
    } else {
        // Plaintext path: existing LOWER() comparison.
        let q = format!("SELECT {USER_COLS} FROM users WHERE LOWER(email) = LOWER($1)");
        sqlx::query_as::<_, User>(&q)
            .bind(email)
            .fetch_optional(pool)
            .await
    }
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
    let user_id = id.to_string();

    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1::uuid")
        .bind(&user_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1::uuid")
        .bind(&user_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM email_verification_tokens WHERE user_id = $1::uuid")
        .bind(&user_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM push_subscriptions WHERE user_id = $1::uuid")
        .bind(&user_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM totp_backup_codes WHERE user_id = $1::uuid")
        .bind(&user_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM note_to_self_channels WHERE user_id = $1::uuid")
        .bind(&user_id)
        .execute(pool)
        .await?;
    sqlx::query(
        "DELETE FROM user_relationships WHERE requester_id = $1::uuid OR addressee_id = $1::uuid",
    )
    .bind(&user_id)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM device_verifications WHERE verifier_id = $1::uuid")
        .bind(&user_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM devices WHERE user_id = $1::uuid")
        .bind(&user_id)
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        UPDATE users SET
            flags = flags | (1 << 5),
            display_name = NULL,
            avatar = NULL,
            banner = NULL,
            bio = NULL,
            status = NULL,
            email = NULL,
            email_hash = NULL,
            password_hash = '',
            totp_secret = NULL,
            totp_enabled = false,
            scheduled_deletion_at = NULL,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $1::uuid
        "#,
    )
    .bind(&user_id)
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

/// Clear any scheduled account deletion for a user.
pub async fn cancel_scheduled_deletion(
    pool: &sqlx::AnyPool,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE users SET scheduled_deletion_at = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = $1::uuid AND scheduled_deletion_at IS NOT NULL",
    )
    .bind(id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
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
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT id::text FROM users \
         WHERE scheduled_deletion_at IS NOT NULL \
           AND scheduled_deletion_at <= NOW() \
           AND (flags & (1 << 5)) = 0",
    )
    .fetch_all(pool)
    .await?;

    let mut affected = 0u64;
    for (user_id,) in rows {
        let uid = user_id
            .parse::<Uuid>()
            .map_err(|_| sqlx::Error::ColumnDecode {
                index: "id".to_string(),
                source: Box::new(std::fmt::Error),
            })?;
        soft_delete_user(pool, uid).await?;
        affected += 1;
    }

    Ok(affected)
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

/// List all local (non-remote) users with pagination for the admin dashboard.
pub async fn list_users(
    pool: &sqlx::AnyPool,
    limit: i64,
    offset: i64,
    search: Option<&str>,
) -> Result<Vec<User>, sqlx::Error> {
    use crate::select_cols::USER_COLS;
    let q = if let Some(q) = search {
        let pattern = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
        let sql = format!(
            "SELECT {USER_COLS} FROM users \
             WHERE is_remote = FALSE AND (username ILIKE $1 OR display_name ILIKE $1) \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        );
        sqlx::query_as::<_, User>(&sql)
            .bind(pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
    } else {
        let sql = format!(
            "SELECT {USER_COLS} FROM users \
             WHERE is_remote = FALSE \
             ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        );
        sqlx::query_as::<_, User>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
    };
    Ok(q)
}

/// Set specific flag bits on a user account (OR in the bits).
pub async fn add_user_flags(pool: &sqlx::AnyPool, id: Uuid, flags: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET flags = flags | $1, updated_at = NOW() WHERE id = $2::uuid")
        .bind(flags)
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Clear specific flag bits on a user account (AND NOT the bits).
pub async fn remove_user_flags(
    pool: &sqlx::AnyPool,
    id: Uuid,
    flags: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET flags = flags & ~$1, updated_at = NOW() WHERE id = $2::uuid")
        .bind(flags)
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}
