//! Member repository — server membership management.

use nexus_common::models::member::Member;
use sqlx::PgPool;
use uuid::Uuid;

/// Add a user as a member of a server.
pub async fn add_member(
    pool: &PgPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<Member, sqlx::Error> {
    sqlx::query_as::<_, Member>(
        r#"
        INSERT INTO members (user_id, server_id, roles, muted, deafened, joined_at)
        VALUES ($1, $2, ARRAY[]::UUID[], false, false, NOW())
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(server_id)
    .fetch_one(pool)
    .await
}

/// Remove a member from a server.
pub async fn remove_member(
    pool: &PgPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM members WHERE user_id = $1 AND server_id = $2")
        .bind(user_id)
        .bind(server_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get a member by user ID and server ID.
pub async fn find_member(
    pool: &PgPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<Option<Member>, sqlx::Error> {
    sqlx::query_as::<_, Member>(
        "SELECT * FROM members WHERE user_id = $1 AND server_id = $2",
    )
    .bind(user_id)
    .bind(server_id)
    .fetch_optional(pool)
    .await
}

/// List members of a server with pagination.
pub async fn list_members(
    pool: &PgPool,
    server_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Member>, sqlx::Error> {
    sqlx::query_as::<_, Member>(
        r#"
        SELECT * FROM members
        WHERE server_id = $1
        ORDER BY joined_at
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(server_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

/// Update member nickname.
pub async fn update_nickname(
    pool: &PgPool,
    user_id: Uuid,
    server_id: Uuid,
    nickname: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE members SET nickname = $3 WHERE user_id = $1 AND server_id = $2")
        .bind(user_id)
        .bind(server_id)
        .bind(nickname)
        .execute(pool)
        .await?;
    Ok(())
}

/// Add a role to a member.
pub async fn add_role(
    pool: &PgPool,
    user_id: Uuid,
    server_id: Uuid,
    role_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE members SET roles = array_append(roles, $3) WHERE user_id = $1 AND server_id = $2 AND NOT ($3 = ANY(roles))",
    )
    .bind(user_id)
    .bind(server_id)
    .bind(role_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Remove a role from a member.
pub async fn remove_role(
    pool: &PgPool,
    user_id: Uuid,
    server_id: Uuid,
    role_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE members SET roles = array_remove(roles, $3) WHERE user_id = $1 AND server_id = $2",
    )
    .bind(user_id)
    .bind(server_id)
    .bind(role_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Check if a user is a member of a server.
pub async fn is_member(
    pool: &PgPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM members WHERE user_id = $1 AND server_id = $2",
    )
    .bind(user_id)
    .bind(server_id)
    .fetch_optional(pool)
    .await?;
    Ok(result.is_some())
}
