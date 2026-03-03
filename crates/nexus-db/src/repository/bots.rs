//! Bot application repository — CRUD for bot_applications and bot_server_installs.

use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;

use nexus_common::models::bot::{BotApplication, BotServerInstall};
use crate::select_cols::{BOT_COLS, BOT_INSTALL_COLS};

fn row_to_bot(row: &sqlx::any::AnyRow) -> BotApplication {
    BotApplication {
        id: row.try_get::<String, _>("id").unwrap_or_default().parse().unwrap_or_default(),
        owner_id: row.try_get::<String, _>("owner_id").unwrap_or_default().parse().unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").unwrap_or(None),
        avatar: row.try_get("avatar").unwrap_or(None),
        public_key: row.try_get("public_key").unwrap_or_default(),
        redirect_uris: row.try_get::<Option<String>, _>("redirect_uris").unwrap_or(None)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        permissions: row.try_get("permissions").unwrap_or(0),
        verified: row.try_get("verified").unwrap_or(false),
        is_public: row.try_get("is_public").unwrap_or(true),
        interactions_endpoint_url: row.try_get("interactions_endpoint_url").unwrap_or(None),
        flags: row.try_get("flags").unwrap_or(0),
        created_at: crate::any_compat::get_datetime(row, "created_at").unwrap_or_default(),
        updated_at: crate::any_compat::get_datetime(row, "updated_at").unwrap_or_default(),
    }
}

fn row_to_server_install(row: &sqlx::any::AnyRow) -> BotServerInstall {
    BotServerInstall {
        id: row.try_get::<String, _>("id").unwrap_or_default().parse().unwrap_or_default(),
        bot_id: row.try_get::<String, _>("bot_id").unwrap_or_default().parse().unwrap_or_default(),
        server_id: row.try_get::<String, _>("server_id").unwrap_or_default().parse().unwrap_or_default(),
        installed_by: row.try_get::<String, _>("installed_by").unwrap_or_default().parse().unwrap_or_default(),
        scopes: row.try_get::<Option<String>, _>("scopes").unwrap_or(None)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        permissions: row.try_get("permissions").unwrap_or(0),
        installed_at: crate::any_compat::get_datetime(row, "installed_at").unwrap_or_default(),
    }
}

// ============================================================================
// Bot Applications
// ============================================================================

pub async fn get_bot(pool: &sqlx::AnyPool, bot_id: Uuid) -> Result<Option<BotApplication>> {
    let row = sqlx::query(&format!("SELECT {} FROM bot_applications WHERE id = $1", BOT_COLS))
        .bind(bot_id.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_bot))
}

pub async fn get_bots_by_owner(pool: &sqlx::AnyPool, owner_id: Uuid) -> Result<Vec<BotApplication>> {
    let rows = sqlx::query(
        &format!("SELECT {} FROM bot_applications WHERE owner_id = $1 ORDER BY created_at DESC", BOT_COLS),
    )
    .bind(owner_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_bot).collect())
}

pub async fn get_bot_by_token_hash(
    pool: &sqlx::AnyPool,
    token_hash: &str,
) -> Result<Option<BotApplication>> {
    let row = sqlx::query(&format!("SELECT {} FROM bot_applications WHERE token_hash = $1", BOT_COLS))
        .bind(token_hash)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_bot))
}

pub async fn create_bot(
    pool: &sqlx::AnyPool,
    id: Uuid,
    owner_id: Uuid,
    name: &str,
    description: Option<&str>,
    token_hash: &str,
    public_key: &str,
    is_public: bool,
    redirect_uris: &[String],
    interactions_endpoint_url: Option<&str>,
) -> Result<BotApplication> {
    let uris = serde_json::to_string(redirect_uris)?;
    let row = sqlx::query(
        &format!(r#"INSERT INTO bot_applications
               (id, owner_id, name, description, token_hash, public_key, is_public,
                redirect_uris, interactions_endpoint_url)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           RETURNING {}"#, BOT_COLS),
    )
    .bind(id.to_string())
    .bind(owner_id.to_string())
    .bind(name)
    .bind(description)
    .bind(token_hash)
    .bind(public_key)
    .bind(is_public)
    .bind(uris)
    .bind(interactions_endpoint_url)
    .fetch_one(pool)
    .await?;
    Ok(row_to_bot(&row))
}

pub async fn update_bot(
    pool: &sqlx::AnyPool,
    bot_id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    avatar: Option<&str>,
    is_public: Option<bool>,
    redirect_uris: Option<&[String]>,
    interactions_endpoint_url: Option<&str>,
) -> Result<Option<BotApplication>> {
    let uris = redirect_uris.map(|r| serde_json::to_string(r)).transpose()?;
    let row = sqlx::query(
        &format!(r#"UPDATE bot_applications SET
               name        = COALESCE($1, name),
               description = COALESCE($2, description),
               avatar      = COALESCE($3, avatar),
               is_public   = COALESCE($4, is_public),
               redirect_uris = COALESCE($5, redirect_uris),
               interactions_endpoint_url = COALESCE($6, interactions_endpoint_url),
               updated_at  = CURRENT_TIMESTAMP
           WHERE id = $7
           RETURNING {}"#, BOT_COLS),
    )
    .bind(name)
    .bind(description)
    .bind(avatar)
    .bind(is_public)
    .bind(uris)
    .bind(interactions_endpoint_url)
    .bind(bot_id.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(row_to_bot))
}

pub async fn update_bot_token(pool: &sqlx::AnyPool, bot_id: Uuid, new_token_hash: &str) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE bot_applications SET token_hash = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
    )
    .bind(new_token_hash)
    .bind(bot_id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_bot(pool: &sqlx::AnyPool, bot_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM bot_applications WHERE id = $1")
        .bind(bot_id.to_string())
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ============================================================================
// Bot Server Installs
// ============================================================================

pub async fn install_bot_to_server(
    pool: &sqlx::AnyPool,
    bot_id: Uuid,
    server_id: Uuid,
    installed_by: Uuid,
    scopes: &[String],
    permissions: i64,
) -> Result<BotServerInstall> {
    let scopes_json = serde_json::to_string(scopes)?;
    let row = sqlx::query(
        &format!(r#"INSERT INTO bot_server_installs (bot_id, server_id, installed_by, scopes, permissions)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (bot_id, server_id) DO UPDATE
               SET scopes = EXCLUDED.scopes,
                   permissions = EXCLUDED.permissions
           RETURNING {}"#, BOT_INSTALL_COLS),
    )
    .bind(bot_id.to_string())
    .bind(server_id.to_string())
    .bind(installed_by.to_string())
    .bind(scopes_json)
    .bind(permissions)
    .fetch_one(pool)
    .await?;
    Ok(row_to_server_install(&row))
}

pub async fn get_server_bots(pool: &sqlx::AnyPool, server_id: Uuid) -> Result<Vec<BotServerInstall>> {
    let rows = sqlx::query(
        &format!("SELECT {} FROM bot_server_installs WHERE server_id = $1 ORDER BY installed_at DESC", BOT_INSTALL_COLS),
    )
    .bind(server_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_server_install).collect())
}

/// Return all servers a given bot is installed in.
pub async fn get_bot_servers(pool: &sqlx::AnyPool, bot_id: Uuid) -> Result<Vec<BotServerInstall>> {
    let rows = sqlx::query(
        &format!("SELECT {} FROM bot_server_installs WHERE bot_id = $1 ORDER BY installed_at DESC", BOT_INSTALL_COLS),
    )
    .bind(bot_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_server_install).collect())
}

pub async fn uninstall_bot_from_server(
    pool: &sqlx::AnyPool,
    bot_id: Uuid,
    server_id: Uuid,
) -> Result<bool> {
    let result = sqlx::query(
        "DELETE FROM bot_server_installs WHERE bot_id = $1 AND server_id = $2",
    )
    .bind(bot_id.to_string())
    .bind(server_id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn is_bot_in_server(pool: &sqlx::AnyPool, bot_id: Uuid, server_id: Uuid) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bot_server_installs WHERE bot_id = $1 AND server_id = $2",
    )
    .bind(bot_id.to_string())
    .bind(server_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}
