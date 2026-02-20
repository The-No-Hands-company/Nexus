//! User repository — CRUD operations for user accounts.

use nexus_common::models::user::User;

use uuid::Uuid;

/// Create a new user account.
pub async fn create_user(
    pool: &sqlx::AnyPool,
    id: Uuid,
    username: &str,
    email: Option<&str>,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, username, email, password_hash, presence, flags, created_at, updated_at)
        VALUES (?, ?, ?, ?, 'offline', 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await
}

/// Find a user by their unique ID.
pub async fn find_by_id(pool: &sqlx::AnyPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

/// Find a user by username (case-insensitive).
pub async fn find_by_username(pool: &sqlx::AnyPool, username: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE LOWER(username) = LOWER(?)")
        .bind(username)
        .fetch_optional(pool)
        .await
}

/// Find a user by email.
pub async fn find_by_email(pool: &sqlx::AnyPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE LOWER(email) = LOWER(?)")
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
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users SET
            username = COALESCE(?, username),
            display_name = COALESCE(?, display_name),
            bio = COALESCE(?, bio),
            status = COALESCE(?, status),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(username)
    .bind(display_name)
    .bind(bio)
    .bind(status)
    .fetch_one(pool)
    .await
}

/// Update user presence state.
pub async fn update_presence(
    pool: &sqlx::AnyPool,
    id: Uuid,
    presence: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET presence = ?::user_presence WHERE id = ?")
        .bind(id.to_string())
        .bind(presence)
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
        WHERE id = ?
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
