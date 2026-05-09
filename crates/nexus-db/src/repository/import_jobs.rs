//! Repository for data portability/import jobs.

use nexus_common::models::ecosystem::ImportJob;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::IMPORT_JOB_COLS;

pub async fn create_import_job(
    pool: &AnyPool,
    id: Uuid,
    server_id: Uuid,
    user_id: Uuid,
    source_platform: &str,
    metadata: &serde_json::Value,
) -> Result<ImportJob, sqlx::Error> {
    let q = format!(
        "INSERT INTO import_jobs (id, server_id, user_id, source_platform, metadata) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING {IMPORT_JOB_COLS}"
    );
    sqlx::query_as::<_, ImportJob>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(user_id.to_string())
        .bind(source_platform)
        .bind(serde_json::to_string(metadata).unwrap_or_default())
        .fetch_one(pool)
        .await
}

pub async fn update_import_progress(
    pool: &AnyPool,
    id: Uuid,
    status: &str,
    imported_items: i32,
    total_items: i32,
    error_log: Option<&str>,
) -> Result<Option<ImportJob>, sqlx::Error> {
    let q = format!(
        "UPDATE import_jobs SET status = $2, imported_items = $3, total_items = $4, \
         error_log = $5, updated_at = now() \
         WHERE id = $1 \
         RETURNING {IMPORT_JOB_COLS}"
    );
    sqlx::query_as::<_, ImportJob>(&q)
        .bind(id.to_string())
        .bind(status)
        .bind(imported_items)
        .bind(total_items)
        .bind(error_log)
        .fetch_optional(pool)
        .await
}

pub async fn list_import_jobs(
    pool: &AnyPool,
    server_id: Uuid,
    limit: i64,
) -> Result<Vec<ImportJob>, sqlx::Error> {
    let q = format!(
        "SELECT {IMPORT_JOB_COLS} FROM import_jobs \
         WHERE server_id = $1 ORDER BY created_at DESC LIMIT $2"
    );
    sqlx::query_as::<_, ImportJob>(&q)
        .bind(server_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn get_import_job(pool: &AnyPool, id: Uuid) -> Result<Option<ImportJob>, sqlx::Error> {
    let q = format!("SELECT {IMPORT_JOB_COLS} FROM import_jobs WHERE id = $1");
    sqlx::query_as::<_, ImportJob>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

/// List pending import jobs across all servers.
pub async fn list_pending_import_jobs(
    pool: &AnyPool,
    limit: i64,
) -> Result<Vec<ImportJob>, sqlx::Error> {
    let q = format!(
        "SELECT {IMPORT_JOB_COLS} FROM import_jobs \
         WHERE status = 'pending' \
         ORDER BY created_at ASC LIMIT $1"
    );
    sqlx::query_as::<_, ImportJob>(&q)
        .bind(limit)
        .fetch_all(pool)
        .await
}

/// Attempt to claim a pending import job for processing.
///
/// Returns `true` if this caller successfully transitioned the job from
/// `pending` → `processing`, `false` if another worker already claimed it.
pub async fn try_claim_import_job(pool: &AnyPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_jobs \
         SET status = 'processing', updated_at = CURRENT_TIMESTAMP \
         WHERE id = $1 AND status = 'pending'",
    )
    .bind(id.to_string())
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Mark an import job as completed with final counters.
pub async fn mark_import_completed(
    pool: &AnyPool,
    id: Uuid,
    imported_items: i32,
    total_items: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE import_jobs \
         SET status = 'completed', imported_items = $2, total_items = $3, \
             error_log = NULL, updated_at = CURRENT_TIMESTAMP \
         WHERE id = $1",
    )
    .bind(id.to_string())
    .bind(imported_items)
    .bind(total_items)
    .execute(pool)
    .await?;
    Ok(())
}

/// Mark an import job as failed with error context and partial progress.
pub async fn mark_import_failed(
    pool: &AnyPool,
    id: Uuid,
    imported_items: i32,
    total_items: i32,
    error_log: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE import_jobs \
         SET status = 'failed', imported_items = $2, total_items = $3, \
             error_log = $4, updated_at = CURRENT_TIMESTAMP \
         WHERE id = $1",
    )
    .bind(id.to_string())
    .bind(imported_items)
    .bind(total_items)
    .bind(error_log)
    .execute(pool)
    .await?;
    Ok(())
}
