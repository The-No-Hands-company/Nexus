//! Member repository — server membership management.

use nexus_common::models::member::Member;

use uuid::Uuid;

use crate::select_cols::MEMBER_COLS;

/// Add a user as a member of a server.
pub async fn add_member(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<Member, sqlx::Error> {
    let q = format!(
        "INSERT INTO members (user_id, server_id, roles, muted, deafened, joined_at) \
         VALUES ($1::uuid, $2::uuid, ARRAY[]::UUID[], false, false, CURRENT_TIMESTAMP) \
         RETURNING {MEMBER_COLS}"
    );
    sqlx::query_as::<_, Member>(&q)
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
    sqlx::query("DELETE FROM members WHERE user_id = $1::uuid AND server_id = $2::uuid")
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
    let q = format!(
        "SELECT {MEMBER_COLS} FROM members WHERE user_id = $1::uuid AND server_id = $2::uuid"
    );
    sqlx::query_as::<_, Member>(&q)
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
    let q = format!(
        "SELECT {MEMBER_COLS} FROM members \
         WHERE server_id = $1::uuid \
         ORDER BY joined_at \
         LIMIT $2 OFFSET $3"
    );
    sqlx::query_as::<_, Member>(&q)
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
    sqlx::query(
        "UPDATE members SET nickname = $1 WHERE user_id = $2::uuid AND server_id = $3::uuid",
    )
    .bind(nickname)
    .bind(user_id.to_string())
    .bind(server_id.to_string())
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
        "UPDATE members SET roles = array_append(roles, $1::uuid) WHERE user_id = $2::uuid AND server_id = $3::uuid AND NOT ($1::uuid = ANY(roles))",
    )
    .bind(role_id.to_string())
    .bind(user_id.to_string())
    .bind(server_id.to_string())
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
        "UPDATE members SET roles = array_remove(roles, $1::uuid) WHERE user_id = $2::uuid AND server_id = $3::uuid",
    )
    .bind(role_id.to_string())
    .bind(user_id.to_string())
    .bind(server_id.to_string())
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
        "SELECT EXISTS(SELECT 1 FROM members WHERE user_id = $1::uuid AND server_id = $2::uuid)",
    )
    .bind(user_id.to_string())
    .bind(server_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(result.0)
}

/// List all server IDs the user is currently a member of.
pub async fn list_server_ids_for_user(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT server_id::text FROM members WHERE user_id = $1::uuid")
            .bind(user_id.to_string())
            .fetch_all(pool)
            .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(s,)| s.parse::<Uuid>().ok())
        .collect())
}
