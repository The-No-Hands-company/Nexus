//! Role repository.

use nexus_common::models::role::Role;

use uuid::Uuid;

use crate::select_cols::ROLE_COLS;

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
    let q = format!(
        "INSERT INTO roles (id, server_id, name, color, hoist, position, permissions, mentionable, is_default, created_at, updated_at) \
         VALUES ($1::uuid, $2::uuid, $3, $4, false, $5, $6, true, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
         RETURNING {ROLE_COLS}"
    );
    sqlx::query_as::<_, Role>(&q)
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
    let q = format!("SELECT {ROLE_COLS} FROM roles WHERE server_id = $1::uuid ORDER BY position DESC");
    sqlx::query_as::<_, Role>(&q)
    .bind(server_id.to_string())
    .fetch_all(pool)
    .await
}

/// Find a role by ID.
pub async fn find_by_id(pool: &sqlx::AnyPool, id: Uuid) -> Result<Option<Role>, sqlx::Error> {
    let q = format!("SELECT {ROLE_COLS} FROM roles WHERE id = $1::uuid");
    sqlx::query_as::<_, Role>(&q)
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
    let q = format!(
        "UPDATE roles SET \
             name = COALESCE($1, name), \
             color = COALESCE($2, color), \
             permissions = COALESCE($3, permissions), \
             position = COALESCE($4, position), \
             hoist = COALESCE($5, hoist), \
             mentionable = COALESCE($6, mentionable), \
             updated_at = CURRENT_TIMESTAMP \
         WHERE id = $7::uuid \
         RETURNING {ROLE_COLS}"
    );
    sqlx::query_as::<_, Role>(&q)
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
    sqlx::query("DELETE FROM roles WHERE id = $1::uuid AND is_default = false")
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
    let q = format!("SELECT {ROLE_COLS} FROM roles WHERE server_id = $1::uuid AND is_default = true");
    sqlx::query_as::<_, Role>(&q)
    .bind(server_id.to_string())
    .fetch_optional(pool)
    .await
}
