//! Repository for server templates.

use nexus_common::models::ecosystem::ServerTemplate;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::SERVER_TEMPLATE_COLS;

pub async fn create_template(
    pool: &AnyPool,
    id: Uuid,
    name: &str,
    description: Option<&str>,
    category: &str,
    icon_url: Option<&str>,
    channels: &serde_json::Value,
    roles: &serde_json::Value,
    settings: &serde_json::Value,
    is_builtin: bool,
    creator_id: Option<Uuid>,
) -> Result<ServerTemplate, sqlx::Error> {
    let q = format!(
        "INSERT INTO server_templates \
         (id, name, description, category, icon_url, channels, roles, settings, is_builtin, creator_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         RETURNING {SERVER_TEMPLATE_COLS}"
    );
    sqlx::query_as::<_, ServerTemplate>(&q)
        .bind(id.to_string())
        .bind(name)
        .bind(description)
        .bind(category)
        .bind(icon_url)
        .bind(serde_json::to_string(channels).unwrap_or_default())
        .bind(serde_json::to_string(roles).unwrap_or_default())
        .bind(serde_json::to_string(settings).unwrap_or_default())
        .bind(is_builtin)
        .bind(creator_id.map(|u| u.to_string()))
        .fetch_one(pool)
        .await
}

pub async fn list_templates(
    pool: &AnyPool,
    category: Option<&str>,
    limit: i64,
) -> Result<Vec<ServerTemplate>, sqlx::Error> {
    let q = if let Some(cat) = category {
        let q = format!(
            "SELECT {SERVER_TEMPLATE_COLS} FROM server_templates \
             WHERE category = $1 ORDER BY usage_count DESC LIMIT $2"
        );
        return sqlx::query_as::<_, ServerTemplate>(&q)
            .bind(cat)
            .bind(limit)
            .fetch_all(pool)
            .await;
    } else {
        format!(
            "SELECT {SERVER_TEMPLATE_COLS} FROM server_templates \
             ORDER BY usage_count DESC LIMIT $1"
        )
    };
    sqlx::query_as::<_, ServerTemplate>(&q)
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn get_template(
    pool: &AnyPool,
    id: Uuid,
) -> Result<Option<ServerTemplate>, sqlx::Error> {
    let q = format!(
        "SELECT {SERVER_TEMPLATE_COLS} FROM server_templates WHERE id = $1"
    );
    sqlx::query_as::<_, ServerTemplate>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn increment_usage(pool: &AnyPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE server_templates SET usage_count = usage_count + 1 WHERE id = $1")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_template(pool: &AnyPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query("DELETE FROM server_templates WHERE id = $1")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
