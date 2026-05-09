//! Scylla dual-write outbox for message mutations.
//!
//! This provides a durable queue of message writes/deletes that can be
//! consumed by a Scylla sync worker.

use crate::search::MessageDocument;
use sqlx::AnyPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OutboxJob {
    pub id: i64,
    pub operation: String,
    pub message_id: Uuid,
    pub payload: Option<serde_json::Value>,
    pub attempt_count: i32,
}

pub async fn enqueue_upsert_message(
    pool: &AnyPool,
    doc: &MessageDocument,
) -> Result<(), sqlx::Error> {
    let payload = serde_json::to_value(doc).unwrap_or(serde_json::Value::Null);

    sqlx::query(
        "INSERT INTO scylla_message_outbox \
            (operation, message_id, payload, status, attempt_count, created_at, updated_at) \
         VALUES ('upsert', $1::uuid, $2::jsonb, 'pending', 0, NOW(), NOW())",
    )
    .bind(doc.id.as_str())
    .bind(payload.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn enqueue_delete_message(pool: &AnyPool, message_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO scylla_message_outbox \
            (operation, message_id, payload, status, attempt_count, created_at, updated_at) \
         VALUES ('delete', $1::uuid, NULL, 'pending', 0, NOW(), NOW())",
    )
    .bind(message_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn claim_pending_jobs(pool: &AnyPool, limit: i64) -> Result<Vec<OutboxJob>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (i64, String, String, Option<String>, i32)>(
        "WITH claimed AS ( \
            SELECT id \
            FROM scylla_message_outbox \
            WHERE (status = 'pending' OR (status = 'dispatched' AND updated_at < NOW() - INTERVAL '2 minutes')) \
              AND (attempt_count < 10) \
            ORDER BY created_at ASC \
            LIMIT $1 \
            FOR UPDATE SKIP LOCKED \
        ) \
        UPDATE scylla_message_outbox o \
        SET status = 'dispatched', \
            attempt_count = attempt_count + 1, \
            updated_at = NOW() \
        FROM claimed \
        WHERE o.id = claimed.id \
        RETURNING o.id, o.operation, o.message_id::text, o.payload::text, o.attempt_count"
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let jobs = rows
        .into_iter()
        .filter_map(|(id, operation, message_id, payload, attempts)| {
            let mid = message_id.parse::<Uuid>().ok()?;
            let payload = payload.and_then(|p| serde_json::from_str::<serde_json::Value>(&p).ok());
            Some(OutboxJob {
                id,
                operation,
                message_id: mid,
                payload,
                attempt_count: attempts,
            })
        })
        .collect();

    Ok(jobs)
}

pub async fn mark_job_completed(pool: &AnyPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE scylla_message_outbox \
         SET status = 'processed', dispatched_at = NOW(), updated_at = NOW(), last_error = NULL \
         WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_job_failed(pool: &AnyPool, id: i64, error: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE scylla_message_outbox \
         SET status = CASE WHEN attempt_count >= 10 THEN 'failed' ELSE 'pending' END, \
             last_error = $2, \
             updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(id)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}
