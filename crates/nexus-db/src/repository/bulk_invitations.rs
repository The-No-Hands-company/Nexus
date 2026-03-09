//! Repository for bulk email invitations.

use nexus_common::models::ecosystem::BulkInvitation;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::BULK_INVITATION_COLS;

pub async fn create_bulk_invitation(
    pool: &AnyPool,
    id: Uuid,
    server_id: Uuid,
    inviter_id: Uuid,
    emails: &serde_json::Value,
    total_count: i32,
    invite_code: Option<&str>,
) -> Result<BulkInvitation, sqlx::Error> {
    let q = format!(
        "INSERT INTO bulk_invitations (id, server_id, inviter_id, emails, total_count, invite_code) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING {BULK_INVITATION_COLS}"
    );
    sqlx::query_as::<_, BulkInvitation>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(inviter_id.to_string())
        .bind(serde_json::to_string(emails).unwrap_or_default())
        .bind(total_count)
        .bind(invite_code)
        .fetch_one(pool)
        .await
}

pub async fn update_bulk_invitation_status(
    pool: &AnyPool,
    id: Uuid,
    status: &str,
    sent_count: i32,
) -> Result<Option<BulkInvitation>, sqlx::Error> {
    let q = format!(
        "UPDATE bulk_invitations SET status = $2, sent_count = $3 \
         WHERE id = $1 \
         RETURNING {BULK_INVITATION_COLS}"
    );
    sqlx::query_as::<_, BulkInvitation>(&q)
        .bind(id.to_string())
        .bind(status)
        .bind(sent_count)
        .fetch_optional(pool)
        .await
}

pub async fn list_bulk_invitations(
    pool: &AnyPool,
    server_id: Uuid,
    limit: i64,
) -> Result<Vec<BulkInvitation>, sqlx::Error> {
    let q = format!(
        "SELECT {BULK_INVITATION_COLS} FROM bulk_invitations \
         WHERE server_id = $1 ORDER BY created_at DESC LIMIT $2"
    );
    sqlx::query_as::<_, BulkInvitation>(&q)
        .bind(server_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}
