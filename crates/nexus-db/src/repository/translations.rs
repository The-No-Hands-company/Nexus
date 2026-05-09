//! Message translation cache repository queries.

use nexus_common::models::accessibility::MessageTranslation;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::MESSAGE_TRANSLATION_COLS;

/// Get cached translation for a message in a target language.
pub async fn get_translation(
    pool: &AnyPool,
    message_id: Uuid,
    target_language: &str,
) -> Result<Option<MessageTranslation>, sqlx::Error> {
    let q = format!(
        "SELECT {MESSAGE_TRANSLATION_COLS} FROM message_translations \
         WHERE message_id = $1 AND target_language = $2"
    );
    sqlx::query_as::<_, MessageTranslation>(&q)
        .bind(message_id.to_string())
        .bind(target_language)
        .fetch_optional(pool)
        .await
}

/// Store a translation (upsert — updates if same message+language combo exists).
pub async fn upsert_translation(
    pool: &AnyPool,
    id: Uuid,
    message_id: Uuid,
    source_language: &str,
    target_language: &str,
    translated_text: &str,
) -> Result<MessageTranslation, sqlx::Error> {
    let q = format!(
        "INSERT INTO message_translations (id, message_id, source_language, target_language, translated_text) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (message_id, target_language) DO UPDATE SET \
         translated_text = $5, source_language = $3 \
         RETURNING {MESSAGE_TRANSLATION_COLS}"
    );
    sqlx::query_as::<_, MessageTranslation>(&q)
        .bind(id.to_string())
        .bind(message_id.to_string())
        .bind(source_language)
        .bind(target_language)
        .bind(translated_text)
        .fetch_one(pool)
        .await
}

/// List all translations for a message.
pub async fn list_translations(
    pool: &AnyPool,
    message_id: Uuid,
) -> Result<Vec<MessageTranslation>, sqlx::Error> {
    let q = format!(
        "SELECT {MESSAGE_TRANSLATION_COLS} FROM message_translations WHERE message_id = $1"
    );
    sqlx::query_as::<_, MessageTranslation>(&q)
        .bind(message_id.to_string())
        .fetch_all(pool)
        .await
}

/// Delete cached translations for a message (e.g. after message edit).
pub async fn delete_translations(pool: &AnyPool, message_id: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query("DELETE FROM message_translations WHERE message_id = $1")
        .bind(message_id.to_string())
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
