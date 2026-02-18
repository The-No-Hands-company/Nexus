//! Role repository.

use nexus_common::models::role::Role;
use sqlx::PgPool;
use uuid::Uuid;

/// Create a new role.
pub async fn create_role(
    pool: &PgPool,
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
        VALUES ($1, $2, $3, $4, false, $5, $6, true, $7, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(server_id)
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
    pool: &PgPool,
    server_id: Uuid,
) -> Result<Vec<Role>, sqlx::Error> {
    sqlx::query_as::<_, Role>(
        "SELECT * FROM roles WHERE server_id = $1 ORDER BY position DESC",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
}

/// Find a role by ID.
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Role>, sqlx::Error> {
    sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Update a role.
pub async fn update_role(
    pool: &PgPool,
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
            name = COALESCE($2, name),
            color = COALESCE($3, color),
            permissions = COALESCE($4, permissions),
            position = COALESCE($5, position),
            hoist = COALESCE($6, hoist),
            mentionable = COALESCE($7, mentionable),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
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
pub async fn delete_role(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM roles WHERE id = $1 AND is_default = false")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get the @everyone role for a server.
pub async fn get_everyone_role(
    pool: &PgPool,
    server_id: Uuid,
) -> Result<Option<Role>, sqlx::Error> {
    sqlx::query_as::<_, Role>(
        "SELECT * FROM roles WHERE server_id = $1 AND is_default = true",
    )
    .bind(server_id)
    .fetch_optional(pool)
    .await
}
