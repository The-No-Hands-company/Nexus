//! Custom emoji repository — CRUD for server-level custom emoji.

use nexus_common::models::rich::ServerEmojiRow;

use uuid::Uuid;

// Module-level helper rows (sqlx::FromRow cannot be derived on local types)
#[derive(sqlx::FromRow)]
struct StorageKeyRow { storage_key: String }

#[derive(sqlx::FromRow)]
struct CountRow { count: i64 }

// ============================================================
// Create
// ============================================================

/// Insert a new custom emoji for a server.
pub async fn create_emoji(
    pool: &sqlx::AnyPool,
    id: Uuid,
    server_id: Uuid,
    creator_id: Uuid,
    name: &str,
    storage_key: &str,
    url: Option<&str>,
    animated: bool,
) -> Result<ServerEmojiRow, sqlx::Error> {
    sqlx::query_as::<_, ServerEmojiRow>(
        r#"
        INSERT INTO server_emoji (
            id, server_id, creator_id, name,
            storage_key, url, animated,
            managed, available, created_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, false, true, CURRENT_TIMESTAMP)
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(server_id.to_string())
    .bind(creator_id.to_string())
    .bind(name)
    .bind(storage_key)
    .bind(url)
    .bind(animated)
    .fetch_one(pool)
    .await
}

// ============================================================
// Read
// ============================================================

/// Get all emoji for a server.
pub async fn list_for_server(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
) -> Result<Vec<ServerEmojiRow>, sqlx::Error> {
    sqlx::query_as::<_, ServerEmojiRow>(
        "SELECT * FROM server_emoji WHERE server_id = ? ORDER BY name",
    )
    .bind(server_id.to_string())
    .fetch_all(pool)
    .await
}

/// Get a single emoji by ID.
pub async fn find_by_id(
    pool: &sqlx::AnyPool,
    id: Uuid,
) -> Result<Option<ServerEmojiRow>, sqlx::Error> {
    sqlx::query_as::<_, ServerEmojiRow>("SELECT * FROM server_emoji WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

/// Get a single emoji by server + name.
pub async fn find_by_name(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
    name: &str,
) -> Result<Option<ServerEmojiRow>, sqlx::Error> {
    sqlx::query_as::<_, ServerEmojiRow>(
        "SELECT * FROM server_emoji WHERE server_id = ? AND name = ?",
    )
    .bind(server_id.to_string())
    .bind(name)
    .fetch_optional(pool)
    .await
}

// ============================================================
// Update
// ============================================================

/// Rename an emoji.
pub async fn update_emoji(
    pool: &sqlx::AnyPool,
    id: Uuid,
    server_id: Uuid,
    name: &str,
) -> Result<ServerEmojiRow, sqlx::Error> {
    sqlx::query_as::<_, ServerEmojiRow>(
        r#"
        UPDATE server_emoji
        SET name = ?
        WHERE id = ? AND server_id = ?
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(server_id.to_string())
    .bind(name)
    .fetch_one(pool)
    .await
}

/// Set an emoji's public URL after upload.
pub async fn set_url(pool: &sqlx::AnyPool, id: Uuid, url: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE server_emoji SET url = ? WHERE id = ?")
        .bind(id.to_string())
        .bind(url)
        .execute(pool)
        .await?;
    Ok(())
}

// ============================================================
// Delete
// ============================================================

/// Delete an emoji. Returns the storage_key so the caller can clean up storage.
pub async fn delete_emoji(
    pool: &sqlx::AnyPool,
    id: Uuid,
    server_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query_as::<_, StorageKeyRow>(
        "DELETE FROM server_emoji WHERE id = ? AND server_id = ? RETURNING storage_key",
    )
    .bind(id.to_string())
    .bind(server_id.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.storage_key))
}

/// Count emoji for a server (for limit enforcement).
pub async fn count_for_server(pool: &sqlx::AnyPool, server_id: Uuid) -> Result<i64, sqlx::Error> {
    let row = sqlx::query_as::<_, CountRow>(
        "SELECT COUNT(*) AS count FROM server_emoji WHERE server_id = ?",
    )
    .bind(server_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(row.count)
}
