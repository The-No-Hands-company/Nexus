//! Webhook repository — incoming and outgoing webhooks.

use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use nexus_common::models::webhook::{Webhook, WebhookType};

fn row_to_webhook(row: &sqlx::postgres::PgRow) -> Webhook {
    let wt: String = row.try_get("webhook_type").unwrap_or_default();
    let webhook_type = match wt.as_str() {
        "outgoing" => WebhookType::Outgoing,
        _ => WebhookType::Incoming,
    };
    Webhook {
        id: row.try_get("id").unwrap(),
        webhook_type,
        server_id: row.try_get("server_id").unwrap_or(None),
        channel_id: row.try_get("channel_id").unwrap_or(None),
        creator_id: row.try_get("creator_id").unwrap_or(None),
        name: row.try_get("name").unwrap(),
        avatar: row.try_get("avatar").unwrap_or(None),
        token: row.try_get("token").unwrap_or(None),
        url: row.try_get("url").unwrap_or(None),
        events: {
            let v: Option<serde_json::Value> = row.try_get("events").unwrap_or(None);
            v.and_then(|j| serde_json::from_value(j).ok()).unwrap_or_default()
        },
        active: row.try_get("active").unwrap_or(true),
        delivery_count: row.try_get("delivery_count").unwrap_or(0),
        created_at: row.try_get("created_at").unwrap(),
        updated_at: row.try_get("updated_at").unwrap(),
    }
}

pub async fn get_webhook(pool: &PgPool, webhook_id: Uuid) -> Result<Option<Webhook>> {
    let row = sqlx::query("SELECT * FROM webhooks WHERE id = $1")
        .bind(webhook_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_webhook))
}

pub async fn get_webhook_by_token(
    pool: &PgPool,
    webhook_id: Uuid,
    token: &str,
) -> Result<Option<Webhook>> {
    let row = sqlx::query("SELECT * FROM webhooks WHERE id = $1 AND token = $2")
        .bind(webhook_id)
        .bind(token)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_webhook))
}

pub async fn get_channel_webhooks(pool: &PgPool, channel_id: Uuid) -> Result<Vec<Webhook>> {
    let rows = sqlx::query(
        "SELECT * FROM webhooks WHERE channel_id = $1 ORDER BY created_at DESC",
    )
    .bind(channel_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_webhook).collect())
}

pub async fn get_server_webhooks(pool: &PgPool, server_id: Uuid) -> Result<Vec<Webhook>> {
    let rows = sqlx::query(
        "SELECT * FROM webhooks WHERE server_id = $1 ORDER BY created_at DESC",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_webhook).collect())
}

pub async fn create_incoming_webhook(
    pool: &PgPool,
    id: Uuid,
    server_id: Uuid,
    channel_id: Uuid,
    creator_id: Uuid,
    name: &str,
    avatar: Option<&str>,
    token: &str,
) -> Result<Webhook> {
    let row = sqlx::query(
        r#"INSERT INTO webhooks
               (id, server_id, channel_id, creator_id, name, avatar, token, webhook_type)
           VALUES ($1, $2, $3, $4, $5, $6, $7, 'incoming')
           RETURNING *"#,
    )
    .bind(id)
    .bind(server_id)
    .bind(channel_id)
    .bind(creator_id)
    .bind(name)
    .bind(avatar)
    .bind(token)
    .fetch_one(pool)
    .await?;
    Ok(row_to_webhook(&row))
}

pub async fn create_outgoing_webhook(
    pool: &PgPool,
    id: Uuid,
    server_id: Uuid,
    creator_id: Uuid,
    name: &str,
    url: &str,
    events: &[String],
    avatar: Option<&str>,
) -> Result<Webhook> {
    let events_json = serde_json::to_value(events)?;
    let row = sqlx::query(
        r#"INSERT INTO webhooks
               (id, server_id, creator_id, name, url, events, avatar, webhook_type)
           VALUES ($1, $2, $3, $4, $5, $6, $7, 'outgoing')
           RETURNING *"#,
    )
    .bind(id)
    .bind(server_id)
    .bind(creator_id)
    .bind(name)
    .bind(url)
    .bind(events_json)
    .bind(avatar)
    .fetch_one(pool)
    .await?;
    Ok(row_to_webhook(&row))
}

pub async fn update_webhook(
    pool: &PgPool,
    webhook_id: Uuid,
    name: Option<&str>,
    avatar: Option<&str>,
    channel_id: Option<Uuid>,
    url: Option<&str>,
    events: Option<&[String]>,
    active: Option<bool>,
) -> Result<Option<Webhook>> {
    let events_json = events.map(serde_json::to_value).transpose()?;
    let row = sqlx::query(
        r#"UPDATE webhooks SET
               name       = COALESCE($2, name),
               avatar     = COALESCE($3, avatar),
               channel_id = COALESCE($4, channel_id),
               url        = COALESCE($5, url),
               events     = COALESCE($6, events),
               active     = COALESCE($7, active),
               updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(webhook_id)
    .bind(name)
    .bind(avatar)
    .bind(channel_id)
    .bind(url)
    .bind(events_json)
    .bind(active)
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(row_to_webhook))
}

pub async fn delete_webhook(pool: &PgPool, webhook_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM webhooks WHERE id = $1")
        .bind(webhook_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
