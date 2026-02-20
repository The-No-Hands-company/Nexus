//! Member repository — server membership management.

use nexus_common::models::member::Member;

use uuid::Uuid;

/// Add a user as a member of a server.
pub async fn add_member(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<Member, sqlx::Error> {
    sqlx::query_as::<_, Member>(
        r#"
        INSERT INTO members (user_id, server_id, roles, muted, deafened, joined_at)
        VALUES (?, ?, ARRAY[]::UUID[], false, false, CURRENT_TIMESTAMP)
        RETURNING *
        "#,
    )
    .bind(user_id.to_string())
    .bind(server_id.to_string())
    .fetch_one(pool)
    .await
}

/// Remove a member from a server.
pub async fn remove_member(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM members WHERE user_id = ? AND server_id = ?")
        .bind(user_id.to_string())
        .bind(server_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Get a member by user ID and server ID.
pub async fn find_member(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<Option<Member>, sqlx::Error> {
    sqlx::query_as::<_, Member>(
        "SELECT * FROM members WHERE user_id = ? AND server_id = ?",
    )
    .bind(user_id.to_string())
    .bind(server_id.to_string())
    .fetch_optional(pool)
    .await
}

/// List members of a server with pagination.
pub async fn list_members(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Member>, sqlx::Error> {
    sqlx::query_as::<_, Member>(
        r#"
        SELECT * FROM members
        WHERE server_id = ?
        ORDER BY joined_at
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(server_id.to_string())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

/// Update member nickname.
pub async fn update_nickname(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
    nickname: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE members SET nickname = ? WHERE user_id = ? AND server_id = ?")
        .bind(user_id.to_string())
        .bind(server_id.to_string())
        .bind(nickname)
        .execute(pool)
        .await?;
    Ok(())
}

/// Add a role to a member.
pub async fn add_role(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
    role_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE members SET roles = array_append(roles, ?) WHERE user_id = ? AND server_id = ? AND NOT (? = ANY(roles))",
    )
    .bind(user_id.to_string())
    .bind(server_id.to_string())
    .bind(role_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

/// Remove a role from a member.
pub async fn remove_role(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
    role_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE members SET roles = array_remove(roles, ?) WHERE user_id = ? AND server_id = ?",
    )
    .bind(user_id.to_string())
    .bind(server_id.to_string())
    .bind(role_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

/// Check if a user is a member of a server.
pub async fn is_member(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM members WHERE user_id = ? AND server_id = ?)",
    )
    .bind(user_id.to_string())
    .bind(server_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(result.0)
}
