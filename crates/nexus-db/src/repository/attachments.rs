//! Attachment repository — file upload metadata CRUD.
//!
//! Files are stored in MinIO/S3; this table tracks the metadata
//! (filename, size, content type, storage key, etc.).

use nexus_common::models::rich::AttachmentRow;

use uuid::Uuid;

// ============================================================
// Create
// ============================================================

/// Insert a new pending attachment record.
#[allow(clippy::too_many_arguments)]
pub async fn create_attachment(
    pool: &sqlx::AnyPool,
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
            ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?,
            ?, ?, 'pending',
            CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        )
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(uploader_id.to_string())
    .bind(server_id.map(|u| u.to_string()))
    .bind(channel_id.map(|u| u.to_string()))
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
pub async fn find_by_id(pool: &sqlx::AnyPool, id: Uuid) -> Result<Option<AttachmentRow>, sqlx::Error> {
    sqlx::query_as::<_, AttachmentRow>("SELECT * FROM attachments WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

/// Find all attachments for a message.
pub async fn list_for_message(
    pool: &sqlx::AnyPool,
    message_id: Uuid,
) -> Result<Vec<AttachmentRow>, sqlx::Error> {
    sqlx::query_as::<_, AttachmentRow>(
        "SELECT * FROM attachments WHERE message_id = ? ORDER BY created_at",
    )
    .bind(message_id.to_string())
    .fetch_all(pool)
    .await
}

/// Find all attachments uploaded by a user (paginated).
pub async fn list_for_uploader(
    pool: &sqlx::AnyPool,
    uploader_id: Uuid,
    limit: i64,
    before_id: Option<Uuid>,
) -> Result<Vec<AttachmentRow>, sqlx::Error> {
    if let Some(before) = before_id {
        sqlx::query_as::<_, AttachmentRow>(
            r#"
            SELECT a.* FROM attachments a
            WHERE a.uploader_id = ?
              AND a.id < ?
            ORDER BY a.created_at DESC
            LIMIT ?
            "#,
        )
        .bind(uploader_id.to_string())
        .bind(before.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, AttachmentRow>(
            r#"
            SELECT * FROM attachments
            WHERE uploader_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(uploader_id.to_string())
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
    pool: &sqlx::AnyPool,
    id: Uuid,
    url: &str,
    blurhash: Option<&str>,
) -> Result<AttachmentRow, sqlx::Error> {
    sqlx::query_as::<_, AttachmentRow>(
        r#"
        UPDATE attachments
        SET status = 'ready', url = ?, blurhash = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        RETURNING *
        "#,
    )
    .bind(id.to_string())
    .bind(url)
    .bind(blurhash)
    .fetch_one(pool)
    .await
}

/// Link an attachment to a message after the message is created.
pub async fn attach_to_message(
    pool: &sqlx::AnyPool,
    attachment_id: Uuid,
    message_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE attachments SET message_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(attachment_id.to_string())
    .bind(message_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

/// Mark an attachment as failed.
pub async fn mark_failed(pool: &sqlx::AnyPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE attachments SET status = 'failed', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

// ============================================================
// Delete
// ============================================================

/// Delete an attachment record. Caller is responsible for deleting from storage.
pub async fn delete_attachment(
    pool: &sqlx::AnyPool,
    id: Uuid,
    uploader_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM attachments WHERE id = ? AND uploader_id = ?",
    )
    .bind(id.to_string())
    .bind(uploader_id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
