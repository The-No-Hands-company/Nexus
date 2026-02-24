//! Relationship repository — friend requests, friendships, and blocks.

use nexus_common::models::relationship::Relationship;
use uuid::Uuid;

use crate::select_cols::RELATIONSHIP_COLS;

/// Return every relationship row where the given user is requester OR addressee.
pub async fn list_for_user(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
) -> Result<Vec<Relationship>, sqlx::Error> {
    let q = format!(
        "SELECT {RELATIONSHIP_COLS} FROM user_relationships \
         WHERE requester_id = $1::uuid OR addressee_id = $1::uuid \
         ORDER BY created_at DESC"
    );
    sqlx::query_as::<_, Relationship>(&q)
        .bind(user_id.to_string())
        .fetch_all(pool)
        .await
}

/// Return a single relationship between two users (in either direction), if any.
pub async fn find_between(
    pool: &sqlx::AnyPool,
    user_a: Uuid,
    user_b: Uuid,
) -> Result<Option<Relationship>, sqlx::Error> {
    let q = format!(
        "SELECT {RELATIONSHIP_COLS} FROM user_relationships \
         WHERE (requester_id = $1::uuid AND addressee_id = $2::uuid) \
            OR (requester_id = $2::uuid AND addressee_id = $1::uuid)"
    );
    sqlx::query_as::<_, Relationship>(&q)
        .bind(user_a.to_string())
        .bind(user_b.to_string())
        .fetch_optional(pool)
        .await
}

/// Insert a new relationship row (pending friend request or block).
pub async fn create(
    pool: &sqlx::AnyPool,
    id: Uuid,
    requester_id: Uuid,
    addressee_id: Uuid,
    status: &str, // "pending" | "blocked"
) -> Result<Relationship, sqlx::Error> {
    let q = format!(
        "INSERT INTO user_relationships \
             (id, requester_id, addressee_id, status, created_at, updated_at) \
         VALUES ($1::uuid, $2::uuid, $3::uuid, $4::relationship_status, \
                 CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
         RETURNING {RELATIONSHIP_COLS}"
    );
    sqlx::query_as::<_, Relationship>(&q)
        .bind(id.to_string())
        .bind(requester_id.to_string())
        .bind(addressee_id.to_string())
        .bind(status)
        .fetch_one(pool)
        .await
}

/// Update the status of an existing relationship by its id.
pub async fn update_status(
    pool: &sqlx::AnyPool,
    relationship_id: Uuid,
    new_status: &str,
) -> Result<Relationship, sqlx::Error> {
    let q = format!(
        "UPDATE user_relationships \
         SET status = $1::relationship_status, updated_at = CURRENT_TIMESTAMP \
         WHERE id = $2::uuid \
         RETURNING {RELATIONSHIP_COLS}"
    );
    sqlx::query_as::<_, Relationship>(&q)
        .bind(new_status)
        .bind(relationship_id.to_string())
        .fetch_one(pool)
        .await
}

/// Delete a relationship by its id.
pub async fn delete(
    pool: &sqlx::AnyPool,
    relationship_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM user_relationships WHERE id = $1::uuid")
        .bind(relationship_id.to_string())
        .execute(pool)
        .await
        .map(|_| ())
}
