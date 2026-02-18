//! Bot application repository — CRUD for bot_applications and bot_server_installs.

use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use nexus_common::models::bot::{BotApplication, BotServerInstall};

fn row_to_bot(row: &sqlx::postgres::PgRow) -> BotApplication {
    BotApplication {
        id: row.try_get("id").unwrap(),
        owner_id: row.try_get("owner_id").unwrap(),
        name: row.try_get("name").unwrap(),
        description: row.try_get("description").unwrap_or(None),
        avatar: row.try_get("avatar").unwrap_or(None),
        public_key: row.try_get("public_key").unwrap(),
        redirect_uris: {
            let v: Option<serde_json::Value> = row.try_get("redirect_uris").unwrap_or(None);
            v.and_then(|j| serde_json::from_value(j).ok()).unwrap_or_default()
        },
        permissions: row.try_get("permissions").unwrap_or(0),
        verified: row.try_get("verified").unwrap_or(false),
        is_public: row.try_get("is_public").unwrap_or(true),
        interactions_endpoint_url: row.try_get("interactions_endpoint_url").unwrap_or(None),
        flags: row.try_get("flags").unwrap_or(0),
        created_at: row.try_get("created_at").unwrap(),
        updated_at: row.try_get("updated_at").unwrap(),
    }
}

fn row_to_server_install(row: &sqlx::postgres::PgRow) -> BotServerInstall {
    BotServerInstall {
        id: row.try_get("id").unwrap(),
        bot_id: row.try_get("bot_id").unwrap(),
        server_id: row.try_get("server_id").unwrap(),
        installed_by: row.try_get("installed_by").unwrap(),
        scopes: {
            let v: Option<serde_json::Value> = row.try_get("scopes").unwrap_or(None);
            v.and_then(|j| serde_json::from_value(j).ok()).unwrap_or_default()
        },
        permissions: row.try_get("permissions").unwrap_or(0),
        installed_at: row.try_get("installed_at").unwrap(),
    }
}

// ============================================================================
// Bot Applications
// ============================================================================

pub async fn get_bot(pool: &PgPool, bot_id: Uuid) -> Result<Option<BotApplication>> {
    let row = sqlx::query("SELECT * FROM bot_applications WHERE id = $1")
        .bind(bot_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_bot))
}

pub async fn get_bots_by_owner(pool: &PgPool, owner_id: Uuid) -> Result<Vec<BotApplication>> {
    let rows = sqlx::query(
        "SELECT * FROM bot_applications WHERE owner_id = $1 ORDER BY created_at DESC",
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_bot).collect())
}

pub async fn get_bot_by_token_hash(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<BotApplication>> {
    let row = sqlx::query("SELECT * FROM bot_applications WHERE token_hash = $1")
        .bind(token_hash)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_bot))
}

pub async fn create_bot(
    pool: &PgPool,
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
    let uris = serde_json::to_value(redirect_uris)?;
    let row = sqlx::query(
        r#"INSERT INTO bot_applications
               (id, owner_id, name, description, token_hash, public_key, is_public,
                redirect_uris, interactions_endpoint_url)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           RETURNING *"#,
    )
    .bind(id)
    .bind(owner_id)
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
    pool: &PgPool,
    bot_id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    avatar: Option<&str>,
    is_public: Option<bool>,
    redirect_uris: Option<&[String]>,
    interactions_endpoint_url: Option<&str>,
) -> Result<Option<BotApplication>> {
    let uris = redirect_uris.map(serde_json::to_value).transpose()?;
    let row = sqlx::query(
        r#"UPDATE bot_applications SET
               name        = COALESCE($2, name),
               description = COALESCE($3, description),
               avatar      = COALESCE($4, avatar),
               is_public   = COALESCE($5, is_public),
               redirect_uris = COALESCE($6, redirect_uris),
               interactions_endpoint_url = COALESCE($7, interactions_endpoint_url),
               updated_at  = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(bot_id)
    .bind(name)
    .bind(description)
    .bind(avatar)
    .bind(is_public)
    .bind(uris)
    .bind(interactions_endpoint_url)
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(row_to_bot))
}

pub async fn update_bot_token(pool: &PgPool, bot_id: Uuid, new_token_hash: &str) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE bot_applications SET token_hash = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(new_token_hash)
    .bind(bot_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_bot(pool: &PgPool, bot_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM bot_applications WHERE id = $1")
        .bind(bot_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ============================================================================
// Bot Server Installs
// ============================================================================

pub async fn install_bot_to_server(
    pool: &PgPool,
    bot_id: Uuid,
    server_id: Uuid,
    installed_by: Uuid,
    scopes: &[String],
    permissions: i64,
) -> Result<BotServerInstall> {
    let scopes_json = serde_json::to_value(scopes)?;
    let row = sqlx::query(
        r#"INSERT INTO bot_server_installs (bot_id, server_id, installed_by, scopes, permissions)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (bot_id, server_id) DO UPDATE
               SET scopes = EXCLUDED.scopes,
                   permissions = EXCLUDED.permissions
           RETURNING *"#,
    )
    .bind(bot_id)
    .bind(server_id)
    .bind(installed_by)
    .bind(scopes_json)
    .bind(permissions)
    .fetch_one(pool)
    .await?;
    Ok(row_to_server_install(&row))
}

pub async fn get_server_bots(pool: &PgPool, server_id: Uuid) -> Result<Vec<BotServerInstall>> {
    let rows = sqlx::query(
        "SELECT * FROM bot_server_installs WHERE server_id = $1 ORDER BY installed_at DESC",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_server_install).collect())
}

pub async fn uninstall_bot_from_server(
    pool: &PgPool,
    bot_id: Uuid,
    server_id: Uuid,
) -> Result<bool> {
    let result = sqlx::query(
        "DELETE FROM bot_server_installs WHERE bot_id = $1 AND server_id = $2",
    )
    .bind(bot_id)
    .bind(server_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn is_bot_in_server(pool: &PgPool, bot_id: Uuid, server_id: Uuid) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bot_server_installs WHERE bot_id = $1 AND server_id = $2",
    )
    .bind(bot_id)
    .bind(server_id)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}
