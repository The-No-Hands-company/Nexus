//! Attachment repository — file upload metadata CRUD.
//!
//! Files are stored in MinIO/S3; this table tracks the metadata
//! (filename, size, content type, storage key, etc.).

use nexus_common::models::rich::AttachmentRow;
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================
// Create
// ============================================================

/// Insert a new pending attachment record.
#[allow(clippy::too_many_arguments)]
pub async fn create_attachment(
    pool: &PgPool,
    id: Uuid,
    uploader_id: Uuid,
    server_id: Option<Uuid>,
    channel_id: Option<Uuid>,
    filename: &str,
    content_type: &str,
    size: i64,
    storage_key: &str,
    width: Option<i32>,
    height: Option<i32>,
    duration_secs: Option<f64>,
    spoiler: bool,
    sha256: Option<&str>,
) -> Result<AttachmentRow, sqlx::Error> {
    sqlx::query_as::<_, AttachmentRow>(
        r#"
        INSERT INTO attachments (
            id, uploader_id, server_id, channel_id,
            filename, content_type, size, storage_key,
            width, height, duration_secs,
            spoiler, sha256, status,
            created_at, updated_at
        )
        VALUES (
            $1, $2, $3, $4,
            $5, $6, $7, $8,
            $9, $10, $11,
            $12, $13, 'pending',
            NOW(), NOW()
        )
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(uploader_id)
    .bind(server_id)
    .bind(channel_id)
    .bind(filename)
    .bind(content_type)
    .bind(size)
    .bind(storage_key)
    .bind(width)
    .bind(height)
    .bind(duration_secs)
    .bind(spoiler)
    .bind(sha256)
    .fetch_one(pool)
    .await
}

// ============================================================
// Read
// ============================================================

/// Find an attachment by ID.
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<AttachmentRow>, sqlx::Error> {
    sqlx::query_as::<_, AttachmentRow>("SELECT * FROM attachments WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Find all attachments for a message.
pub async fn list_for_message(
    pool: &PgPool,
    message_id: Uuid,
) -> Result<Vec<AttachmentRow>, sqlx::Error> {
    sqlx::query_as::<_, AttachmentRow>(
        "SELECT * FROM attachments WHERE message_id = $1 ORDER BY created_at",
    )
    .bind(message_id)
    .fetch_all(pool)
    .await
}

/// Find all attachments uploaded by a user (paginated).
pub async fn list_for_uploader(
    pool: &PgPool,
    uploader_id: Uuid,
    limit: i64,
    before_id: Option<Uuid>,
) -> Result<Vec<AttachmentRow>, sqlx::Error> {
    if let Some(before) = before_id {
        sqlx::query_as::<_, AttachmentRow>(
            r#"
            SELECT a.* FROM attachments a
            WHERE a.uploader_id = $1
              AND a.id < $2
            ORDER BY a.created_at DESC
            LIMIT $3
            "#,
        )
        .bind(uploader_id)
        .bind(before)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, AttachmentRow>(
            r#"
            SELECT * FROM attachments
            WHERE uploader_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(uploader_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

// ============================================================
// Update
// ============================================================

/// Mark an attachment as ready and set its public URL.
pub async fn mark_ready(
    pool: &PgPool,
    id: Uuid,
    url: &str,
    blurhash: Option<&str>,
) -> Result<AttachmentRow, sqlx::Error> {
    sqlx::query_as::<_, AttachmentRow>(
        r#"
        UPDATE attachments
        SET status = 'ready', url = $2, blurhash = $3, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(url)
    .bind(blurhash)
    .fetch_one(pool)
    .await
}

/// Link an attachment to a message after the message is created.
pub async fn attach_to_message(
    pool: &PgPool,
    attachment_id: Uuid,
    message_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE attachments SET message_id = $2, updated_at = NOW() WHERE id = $1",
    )
    .bind(attachment_id)
    .bind(message_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Mark an attachment as failed.
pub async fn mark_failed(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE attachments SET status = 'failed', updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

// ============================================================
// Delete
// ============================================================

/// Delete an attachment record. Caller is responsible for deleting from storage.
pub async fn delete_attachment(
    pool: &PgPool,
    id: Uuid,
    uploader_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM attachments WHERE id = $1 AND uploader_id = $2",
    )
    .bind(id)
    .bind(uploader_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
