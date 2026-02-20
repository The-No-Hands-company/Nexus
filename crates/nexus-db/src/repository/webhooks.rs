//! Webhook repository — incoming and outgoing webhooks.

use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;

use nexus_common::models::webhook::{Webhook, WebhookType};

fn row_to_webhook(row: &sqlx::any::AnyRow) -> Webhook {
    let wt: String = row.try_get("webhook_type").unwrap_or_default();
    let webhook_type = match wt.as_str() {
        "outgoing" => WebhookType::Outgoing,
        _ => WebhookType::Incoming,
    };
    Webhook {
        id: row.try_get::<String, _>("id").unwrap_or_default().parse().unwrap_or_default(),
        webhook_type,
        server_id: row.try_get::<Option<String>, _>("server_id").unwrap_or(None).and_then(|s| s.parse().ok()),
        channel_id: row.try_get::<Option<String>, _>("channel_id").unwrap_or(None).and_then(|s| s.parse().ok()),
        creator_id: row.try_get::<Option<String>, _>("creator_id").unwrap_or(None).and_then(|s| s.parse().ok()),
        name: row.try_get("name").unwrap_or_default(),
        avatar: row.try_get("avatar").unwrap_or(None),
        token: row.try_get("token").unwrap_or(None),
        url: row.try_get("url").unwrap_or(None),
        events: row.try_get::<Option<String>, _>("events").unwrap_or(None)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        active: row.try_get("active").unwrap_or(true),
        delivery_count: row.try_get("delivery_count").unwrap_or(0),
        created_at: crate::any_compat::get_datetime(row, "created_at").unwrap_or_default(),
        updated_at: crate::any_compat::get_datetime(row, "updated_at").unwrap_or_default(),
    }
}

pub async fn get_webhook(pool: &sqlx::AnyPool, webhook_id: Uuid) -> Result<Option<Webhook>> {
    let row = sqlx::query("SELECT * FROM webhooks WHERE id = ?")
        .bind(webhook_id.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_webhook))
}

pub async fn get_webhook_by_token(
    pool: &sqlx::AnyPool,
    webhook_id: Uuid,
    token: &str,
) -> Result<Option<Webhook>> {
    let row = sqlx::query("SELECT * FROM webhooks WHERE id = ? AND token = ?")
        .bind(webhook_id.to_string())
        .bind(token)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_webhook))
}

pub async fn get_channel_webhooks(pool: &sqlx::AnyPool, channel_id: Uuid) -> Result<Vec<Webhook>> {
    let rows = sqlx::query(
        "SELECT * FROM webhooks WHERE channel_id = ? ORDER BY created_at DESC",
    )
    .bind(channel_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_webhook).collect())
}

pub async fn get_server_webhooks(pool: &sqlx::AnyPool, server_id: Uuid) -> Result<Vec<Webhook>> {
    let rows = sqlx::query(
        "SELECT * FROM webhooks WHERE server_id = ? ORDER BY created_at DESC",
    )
    .bind(server_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_webhook).collect())
}

pub async fn create_incoming_webhook(
    pool: &sqlx::AnyPool,
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
           VALUES (?, ?, ?, ?, ?, ?, ?, 'incoming')
           RETURNING *"#,
    )
    .bind(id.to_string())
    .bind(server_id.to_string())
    .bind(channel_id.to_string())
    .bind(creator_id.to_string())
    .bind(name)
    .bind(avatar)
    .bind(token)
    .fetch_one(pool)
    .await?;
    Ok(row_to_webhook(&row))
}

pub async fn create_outgoing_webhook(
    pool: &sqlx::AnyPool,
    id: Uuid,
    server_id: Uuid,
    creator_id: Uuid,
    name: &str,
    url: &str,
    events: &[String],
    avatar: Option<&str>,
) -> Result<Webhook> {
    let events_json = serde_json::to_string(events)?;
    let row = sqlx::query(
        r#"INSERT INTO webhooks
               (id, server_id, creator_id, name, url, events, avatar, webhook_type)
           VALUES (?, ?, ?, ?, ?, ?, ?, 'outgoing')
           RETURNING *"#,
    )
    .bind(id.to_string())
    .bind(server_id.to_string())
    .bind(creator_id.to_string())
    .bind(name)
    .bind(url)
    .bind(events_json)
    .bind(avatar)
    .fetch_one(pool)
    .await?;
    Ok(row_to_webhook(&row))
}

pub async fn update_webhook(
    pool: &sqlx::AnyPool,
    webhook_id: Uuid,
    name: Option<&str>,
    avatar: Option<&str>,
    channel_id: Option<Uuid>,
    url: Option<&str>,
    events: Option<&[String]>,
    active: Option<bool>,
) -> Result<Option<Webhook>> {
    let events_json = events.map(|e| serde_json::to_string(e).unwrap_or_default());
    let row = sqlx::query(
        r#"UPDATE webhooks SET
               name       = COALESCE(?, name),
               avatar     = COALESCE(?, avatar),
               channel_id = COALESCE(?, channel_id),
               url        = COALESCE(?, url),
               events     = COALESCE(?, events),
               active     = COALESCE(?, active),
               updated_at = CURRENT_TIMESTAMP
           WHERE id = ?
           RETURNING *"#,
    )
    .bind(name)
    .bind(avatar)
    .bind(channel_id.map(|u| u.to_string()))
    .bind(url)
    .bind(events_json)
    .bind(active)
    .bind(webhook_id.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(row_to_webhook))
}

pub async fn delete_webhook(pool: &sqlx::AnyPool, webhook_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM webhooks WHERE id = ?")
        .bind(webhook_id.to_string())
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
