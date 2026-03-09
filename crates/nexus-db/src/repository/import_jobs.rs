//! Repository for data import jobs (Discord, Slack, Matrix).

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

pub async fn get_import_job(
    pool: &AnyPool,
    id: Uuid,
) -> Result<Option<ImportJob>, sqlx::Error> {
    let q = format!(
        "SELECT {IMPORT_JOB_COLS} FROM import_jobs WHERE id = $1"
    );
    sqlx::query_as::<_, ImportJob>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}
