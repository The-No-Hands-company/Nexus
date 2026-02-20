//! Role repository.

use nexus_common::models::role::Role;

use uuid::Uuid;

/// Create a new role.
pub async fn create_role(
    pool: &sqlx::AnyPool,
    id: Uuid,
    server_id: Uuid,
    name: &str,
    color: Option<i32>,
    permissions: i64,
    position: i32,
    is_default: bool,
) -> Result<Role, sqlx::Error> {
    sqlx::query_as::<_, Role>(
        r#"
        INSERT INTO roles (id, server_id, name, color, hoist, position, permissions, mentionable, is_default, created_at, updated_at)
        VALUES (?, ?, ?, ?, false, ?, ?, true, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(server_id.to_string())
    .bind(name)
    .bind(color)
    .bind(position)
    .bind(permissions)
    .bind(is_default)
    .fetch_one(pool)
    .await
}

/// List all roles in a server.
pub async fn list_server_roles(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
) -> Result<Vec<Role>, sqlx::Error> {
    sqlx::query_as::<_, Role>(
        "SELECT * FROM roles WHERE server_id = ? ORDER BY position DESC",
    )
    .bind(server_id.to_string())
    .fetch_all(pool)
    .await
}

/// Find a role by ID.
pub async fn find_by_id(pool: &sqlx::AnyPool, id: Uuid) -> Result<Option<Role>, sqlx::Error> {
    sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

/// Update a role.
pub async fn update_role(
    pool: &sqlx::AnyPool,
    id: Uuid,
    name: Option<&str>,
    color: Option<i32>,
    permissions: Option<i64>,
    position: Option<i32>,
    hoist: Option<bool>,
    mentionable: Option<bool>,
) -> Result<Role, sqlx::Error> {
    sqlx::query_as::<_, Role>(
        r#"
        UPDATE roles SET
            name = COALESCE(?, name),
            color = COALESCE(?, color),
            permissions = COALESCE(?, permissions),
            position = COALESCE(?, position),
            hoist = COALESCE(?, hoist),
            mentionable = COALESCE(?, mentionable),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(name)
    .bind(color)
    .bind(permissions)
    .bind(position)
    .bind(hoist)
    .bind(mentionable)
    .fetch_one(pool)
    .await
}

/// Delete a role.
pub async fn delete_role(pool: &sqlx::AnyPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM roles WHERE id = ? AND is_default = false")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Get the @everyone role for a server.
pub async fn get_everyone_role(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
) -> Result<Option<Role>, sqlx::Error> {
    sqlx::query_as::<_, Role>(
        "SELECT * FROM roles WHERE server_id = ? AND is_default = true",
    )
    .bind(server_id.to_string())
    .fetch_optional(pool)
    .await
}
