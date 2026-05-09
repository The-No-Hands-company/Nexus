//! In-chat drawing & annotation repository queries.

use nexus_common::models::multimedia::Drawing;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::DRAWING_COLS;

pub async fn create_drawing(
    pool: &AnyPool,
    id: Uuid,
    channel_id: Uuid,
    author_id: Uuid,
    drawing_data: &str,
    width: i32,
    height: i32,
    preview_key: Option<&str>,
    message_id: Option<Uuid>,
    is_whiteboard: bool,
) -> Result<Drawing, sqlx::Error> {
    let q = format!(
        "INSERT INTO drawings (id, channel_id, author_id, drawing_data, width, height, \
         preview_key, message_id, is_whiteboard) \
         VALUES ($1, $2, $3, $4::jsonb, $5, $6, $7, $8, $9) \
         RETURNING {DRAWING_COLS}"
    );
    sqlx::query_as::<_, Drawing>(&q)
        .bind(id.to_string())
        .bind(channel_id.to_string())
        .bind(author_id.to_string())
        .bind(drawing_data)
        .bind(width)
        .bind(height)
        .bind(preview_key)
        .bind(message_id.map(|u| u.to_string()))
        .bind(is_whiteboard)
        .fetch_one(pool)
        .await
}

pub async fn find_drawing(pool: &AnyPool, id: Uuid) -> Result<Option<Drawing>, sqlx::Error> {
    let q = format!("SELECT {DRAWING_COLS} FROM drawings WHERE id = $1");
    sqlx::query_as::<_, Drawing>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn list_by_channel(
    pool: &AnyPool,
    channel_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Drawing>, sqlx::Error> {
    let q = format!(
        "SELECT {DRAWING_COLS} FROM drawings \
         WHERE channel_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    );
    sqlx::query_as::<_, Drawing>(&q)
        .bind(channel_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

pub async fn list_whiteboards(
    pool: &AnyPool,
    channel_id: Uuid,
) -> Result<Vec<Drawing>, sqlx::Error> {
    let q = format!(
        "SELECT {DRAWING_COLS} FROM drawings \
         WHERE channel_id = $1 AND is_whiteboard = true ORDER BY updated_at DESC"
    );
    sqlx::query_as::<_, Drawing>(&q)
        .bind(channel_id.to_string())
        .fetch_all(pool)
        .await
}

pub async fn update_drawing_data(
    pool: &AnyPool,
    id: Uuid,
    drawing_data: &str,
    preview_key: Option<&str>,
) -> Result<Drawing, sqlx::Error> {
    let q = format!(
        "UPDATE drawings SET drawing_data = $2::jsonb, \
         preview_key = COALESCE($3, preview_key), \
         updated_at = NOW() \
         WHERE id = $1 RETURNING {DRAWING_COLS}"
    );
    sqlx::query_as::<_, Drawing>(&q)
        .bind(id.to_string())
        .bind(drawing_data)
        .bind(preview_key)
        .fetch_one(pool)
        .await
}

pub async fn delete_drawing(
    pool: &AnyPool,
    id: Uuid,
    author_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query("DELETE FROM drawings WHERE id = $1 AND author_id = $2")
        .bind(id.to_string())
        .bind(author_id.to_string())
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
