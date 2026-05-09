//! Stories & ephemeral status repository queries.

use nexus_common::models::multimedia::{Story, StoryView};
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::STORY_COLS;

// ── Stories ───────────────────────────────────────────────────────────────────

pub async fn create_story(
    pool: &AnyPool,
    id: Uuid,
    author_id: Uuid,
    media_type: &str,
    storage_key: Option<&str>,
    text_content: Option<&str>,
    text_style: Option<&str>,
    visibility: &str,
    duration_hours: i32,
) -> Result<Story, sqlx::Error> {
    let q = format!(
        "INSERT INTO stories (id, author_id, media_type, storage_key, text_content, \
         text_style, expires_at, visibility) \
         VALUES ($1, $2, $3, $4, $5, $6::jsonb, NOW() + ($7 || ' hours')::interval, $8) \
         RETURNING {STORY_COLS}"
    );
    sqlx::query_as::<_, Story>(&q)
        .bind(id.to_string())
        .bind(author_id.to_string())
        .bind(media_type)
        .bind(storage_key)
        .bind(text_content)
        .bind(text_style)
        .bind(duration_hours.to_string())
        .bind(visibility)
        .fetch_one(pool)
        .await
}

/// List active (non-expired) stories by a user.
pub async fn list_active_by_user(
    pool: &AnyPool,
    author_id: Uuid,
) -> Result<Vec<Story>, sqlx::Error> {
    let q = format!(
        "SELECT {STORY_COLS} FROM stories \
         WHERE author_id = $1 AND expires_at > NOW() \
         ORDER BY created_at DESC"
    );
    sqlx::query_as::<_, Story>(&q)
        .bind(author_id.to_string())
        .fetch_all(pool)
        .await
}

/// List active stories from a set of user IDs (friends feed).
pub async fn list_active_feed(
    pool: &AnyPool,
    user_ids: &[Uuid],
) -> Result<Vec<Story>, sqlx::Error> {
    if user_ids.is_empty() {
        return Ok(vec![]);
    }
    // Build a parameterized IN list
    let placeholders: Vec<String> = user_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("${}", i + 1))
        .collect();
    let q = format!(
        "SELECT {STORY_COLS} FROM stories \
         WHERE author_id IN ({}) AND expires_at > NOW() \
         ORDER BY created_at DESC",
        placeholders.join(", ")
    );
    let mut query = sqlx::query_as::<_, Story>(&q);
    for uid in user_ids {
        query = query.bind(uid.to_string());
    }
    query.fetch_all(pool).await
}

pub async fn find_story(pool: &AnyPool, id: Uuid) -> Result<Option<Story>, sqlx::Error> {
    let q = format!("SELECT {STORY_COLS} FROM stories WHERE id = $1");
    sqlx::query_as::<_, Story>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn delete_story(pool: &AnyPool, id: Uuid, author_id: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query("DELETE FROM stories WHERE id = $1 AND author_id = $2")
        .bind(id.to_string())
        .bind(author_id.to_string())
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

/// Delete all expired stories. Returns count of rows deleted.
pub async fn delete_expired(pool: &AnyPool) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM stories WHERE expires_at <= NOW()")
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

// ── Story Views ───────────────────────────────────────────────────────────────

pub async fn record_view(
    pool: &AnyPool,
    story_id: Uuid,
    viewer_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO story_views (story_id, viewer_id) VALUES ($1, $2) \
         ON CONFLICT (story_id, viewer_id) DO NOTHING",
    )
    .bind(story_id.to_string())
    .bind(viewer_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_viewers(pool: &AnyPool, story_id: Uuid) -> Result<Vec<StoryView>, sqlx::Error> {
    sqlx::query_as::<_, StoryView>(
        "SELECT story_id::text AS story_id, viewer_id::text AS viewer_id, \
         viewed_at::text AS viewed_at \
         FROM story_views WHERE story_id = $1 ORDER BY viewed_at",
    )
    .bind(story_id.to_string())
    .fetch_all(pool)
    .await
}

pub async fn view_count(pool: &AnyPool, story_id: Uuid) -> Result<i64, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct CountRow {
        count: i64,
    }
    let row = sqlx::query_as::<_, CountRow>(
        "SELECT COUNT(*) AS count FROM story_views WHERE story_id = $1",
    )
    .bind(story_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(row.count)
}
