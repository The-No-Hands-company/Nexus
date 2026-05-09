//! Media gallery repository queries — browse attachments with saved filters.

use nexus_common::models::multimedia::MediaGalleryFilter;
use nexus_common::models::rich::AttachmentRow;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::{ATTACHMENT_COLS_A, MEDIA_GALLERY_FILTER_COLS};

// ── Gallery Browsing ──────────────────────────────────────────────────────────

/// Query attachments matching gallery filter criteria.
pub async fn browse_media(
    pool: &AnyPool,
    server_id: Option<Uuid>,
    channel_id: Option<Uuid>,
    media_types: &[String],
    date_from: Option<&str>,
    date_to: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AttachmentRow>, sqlx::Error> {
    // Build WHERE clauses dynamically
    let mut conditions = Vec::new();
    let mut bind_idx = 1u32;

    if server_id.is_some() {
        conditions.push(format!(
            "a.message_id IN (SELECT id FROM messages WHERE channel_id IN \
             (SELECT id FROM channels WHERE server_id = ${bind_idx}))"
        ));
        bind_idx += 1;
    }

    if channel_id.is_some() {
        conditions.push(format!(
            "a.message_id IN (SELECT id FROM messages WHERE channel_id = ${bind_idx})"
        ));
        bind_idx += 1;
    }

    // Sanitize media_types: strip SQL LIKE wildcards (%, _) and restrict to
    // alphanumeric + MIME-legal chars before building the LIKE clause.
    let sanitized_types_for_bind: Vec<String> = media_types
        .iter()
        .map(|mt| {
            mt.chars()
                .filter(|&c| c != '%' && c != '_' && c != '\\')
                .take(64)
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|mt| {
            !mt.is_empty()
                && mt
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '+')
        })
        .collect();

    if !sanitized_types_for_bind.is_empty() {
        let type_conditions: Vec<String> = sanitized_types_for_bind
            .iter()
            .map(|_mt| {
                let cond = format!("a.content_type LIKE ${bind_idx} || '/%'");
                bind_idx += 1;
                cond
            })
            .collect();
        conditions.push(format!("({})", type_conditions.join(" OR ")));
    }

    if date_from.is_some() {
        conditions.push(format!("a.created_at >= ${bind_idx}::timestamptz"));
        bind_idx += 1;
    }

    if date_to.is_some() {
        conditions.push(format!("a.created_at <= ${bind_idx}::timestamptz"));
        bind_idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let q = format!(
        "SELECT {ATTACHMENT_COLS_A} FROM attachments a {where_clause} \
         ORDER BY a.created_at DESC LIMIT ${bind_idx} OFFSET ${}",
        bind_idx + 1
    );

    let mut query = sqlx::query_as::<_, AttachmentRow>(&q);

    if let Some(sid) = server_id {
        query = query.bind(sid.to_string());
    }
    if let Some(cid) = channel_id {
        query = query.bind(cid.to_string());
    }
    // Bind the sanitized type strings, not the raw user input
    for mt in &sanitized_types_for_bind {
        query = query.bind(mt.clone());
    }
    if let Some(df) = date_from {
        query = query.bind(df.to_string());
    }
    if let Some(dt) = date_to {
        query = query.bind(dt.to_string());
    }
    query = query.bind(limit).bind(offset);

    query.fetch_all(pool).await
}

// ── Saved Filters ─────────────────────────────────────────────────────────────

pub async fn create_filter(
    pool: &AnyPool,
    id: Uuid,
    user_id: Uuid,
    server_id: Option<Uuid>,
    channel_id: Option<Uuid>,
    name: &str,
    media_types: &[String],
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Result<MediaGalleryFilter, sqlx::Error> {
    let media_types_pg = format!(
        "ARRAY[{}]::text[]",
        media_types
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 6))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let dt_from_idx = 6 + media_types.len();
    let dt_to_idx = dt_from_idx + 1;

    let q = format!(
        "INSERT INTO media_gallery_filters (id, user_id, server_id, channel_id, name, \
         media_types, date_from, date_to) \
         VALUES ($1, $2, $3, $4, $5, {media_types_pg}, \
         ${dt_from_idx}::timestamptz, ${dt_to_idx}::timestamptz) \
         RETURNING {MEDIA_GALLERY_FILTER_COLS}"
    );
    let mut query = sqlx::query_as::<_, MediaGalleryFilter>(&q)
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(server_id.map(|u| u.to_string()))
        .bind(channel_id.map(|u| u.to_string()))
        .bind(name);
    for mt in media_types {
        query = query.bind(mt.clone());
    }
    query = query.bind(date_from).bind(date_to);
    query.fetch_one(pool).await
}

pub async fn list_filters(
    pool: &AnyPool,
    user_id: Uuid,
) -> Result<Vec<MediaGalleryFilter>, sqlx::Error> {
    let q = format!(
        "SELECT {MEDIA_GALLERY_FILTER_COLS} FROM media_gallery_filters \
         WHERE user_id = $1 ORDER BY created_at DESC"
    );
    sqlx::query_as::<_, MediaGalleryFilter>(&q)
        .bind(user_id.to_string())
        .fetch_all(pool)
        .await
}

pub async fn delete_filter(pool: &AnyPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query("DELETE FROM media_gallery_filters WHERE id = $1 AND user_id = $2")
        .bind(id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
