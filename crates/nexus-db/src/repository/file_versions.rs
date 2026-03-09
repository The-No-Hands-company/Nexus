//! File versioning & storage quota repository queries.

use nexus_common::models::collaboration::{FileVersion, ServerStorageQuota};
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::FILE_VERSION_COLS;

pub async fn create_version(
    pool: &AnyPool,
    id: Uuid,
    attachment_id: Uuid,
    uploader_id: Uuid,
    filename: &str,
    content_type: Option<&str>,
    size: i64,
    storage_key: &str,
    sha256: Option<&str>,
    comment: Option<&str>,
) -> Result<FileVersion, sqlx::Error> {
    let q = format!(
        "INSERT INTO file_versions \
         (id, attachment_id, uploader_id, \
          version_number, filename, content_type, size, storage_key, sha256, comment) \
         VALUES ($1, $2, $3, \
                 (SELECT COALESCE(MAX(version_number), 0) + 1 FROM file_versions WHERE attachment_id = $2), \
                 $4, $5, $6, $7, $8, $9) \
         RETURNING {FILE_VERSION_COLS}"
    );
    sqlx::query_as::<_, FileVersion>(&q)
        .bind(id.to_string())
        .bind(attachment_id.to_string())
        .bind(uploader_id.to_string())
        .bind(filename)
        .bind(content_type)
        .bind(size)
        .bind(storage_key)
        .bind(sha256)
        .bind(comment)
        .fetch_one(pool)
        .await
}

pub async fn list_versions(
    pool: &AnyPool,
    attachment_id: Uuid,
) -> Result<Vec<FileVersion>, sqlx::Error> {
    let q = format!(
        "SELECT {FILE_VERSION_COLS} FROM file_versions \
         WHERE attachment_id = $1 ORDER BY version_number DESC"
    );
    sqlx::query_as::<_, FileVersion>(&q)
        .bind(attachment_id.to_string())
        .fetch_all(pool)
        .await
}

pub async fn get_version(
    pool: &AnyPool,
    attachment_id: Uuid,
    version_number: i32,
) -> Result<Option<FileVersion>, sqlx::Error> {
    let q = format!(
        "SELECT {FILE_VERSION_COLS} FROM file_versions \
         WHERE attachment_id = $1 AND version_number = $2"
    );
    sqlx::query_as::<_, FileVersion>(&q)
        .bind(attachment_id.to_string())
        .bind(version_number)
        .fetch_optional(pool)
        .await
}

pub async fn delete_version(
    pool: &AnyPool,
    version_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM file_versions WHERE id = $1")
        .bind(version_id.to_string())
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ── Storage Quotas ────────────────────────────────────────────────────────────

pub async fn get_quota(
    pool: &AnyPool,
    server_id: Uuid,
) -> Result<ServerStorageQuota, sqlx::Error> {
    sqlx::query_as::<_, ServerStorageQuota>(
        "INSERT INTO server_storage_quotas (server_id) VALUES ($1) \
         ON CONFLICT (server_id) DO NOTHING; \
         SELECT server_id::text AS server_id, max_bytes, used_bytes, \
                updated_at::text AS updated_at \
         FROM server_storage_quotas WHERE server_id = $1",
    )
    .bind(server_id.to_string())
    .fetch_one(pool)
    .await
}

pub async fn update_quota_max(
    pool: &AnyPool,
    server_id: Uuid,
    max_bytes: i64,
) -> Result<ServerStorageQuota, sqlx::Error> {
    sqlx::query_as::<_, ServerStorageQuota>(
        "INSERT INTO server_storage_quotas (server_id, max_bytes) VALUES ($1, $2) \
         ON CONFLICT (server_id) DO UPDATE SET max_bytes = EXCLUDED.max_bytes, updated_at = NOW() \
         RETURNING server_id::text AS server_id, max_bytes, used_bytes, \
                   updated_at::text AS updated_at",
    )
    .bind(server_id.to_string())
    .bind(max_bytes)
    .fetch_one(pool)
    .await
}

pub async fn increment_used(
    pool: &AnyPool,
    server_id: Uuid,
    bytes: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO server_storage_quotas (server_id, used_bytes) VALUES ($1, $2) \
         ON CONFLICT (server_id) DO UPDATE SET \
         used_bytes = server_storage_quotas.used_bytes + $2, updated_at = NOW()",
    )
    .bind(server_id.to_string())
    .bind(bytes)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn decrement_used(
    pool: &AnyPool,
    server_id: Uuid,
    bytes: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE server_storage_quotas SET \
         used_bytes = GREATEST(0, used_bytes - $2), updated_at = NOW() \
         WHERE server_id = $1",
    )
    .bind(server_id.to_string())
    .bind(bytes)
    .execute(pool)
    .await?;
    Ok(())
}
