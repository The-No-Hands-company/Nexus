//! Slash command and interaction repository.

use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use nexus_common::models::slash_command::{CommandOption, Interaction, SlashCommand};

fn row_to_command(row: &sqlx::postgres::PgRow) -> SlashCommand {
    let options: Vec<CommandOption> = {
        let v: Option<serde_json::Value> = row.try_get("options").unwrap_or(None);
        v.and_then(|j| serde_json::from_value(j).ok()).unwrap_or_default()
    };
    SlashCommand {
        id: row.try_get("id").unwrap(),
        application_id: row.try_get("application_id").unwrap(),
        server_id: row.try_get("server_id").unwrap_or(None),
        name: row.try_get("name").unwrap(),
        name_localizations: row.try_get("name_localizations").unwrap_or(None),
        description: row.try_get("description").unwrap_or_default(),
        description_localizations: row.try_get("description_localizations").unwrap_or(None),
        options,
        default_member_permissions: row.try_get("default_member_permissions").unwrap_or(None),
        dm_permission: row.try_get("dm_permission").unwrap_or(true),
        command_type: row.try_get("command_type").unwrap_or(1),
        version: row.try_get("version").unwrap(),
        enabled: row.try_get("enabled").unwrap_or(true),
        created_at: row.try_get("created_at").unwrap(),
        updated_at: row.try_get("updated_at").unwrap(),
    }
}

fn row_to_interaction(row: &sqlx::postgres::PgRow) -> Interaction {
    Interaction {
        id: row.try_get("id").unwrap(),
        application_id: row.try_get("application_id").unwrap(),
        interaction_type: row.try_get("interaction_type").unwrap_or_else(|_| "APPLICATION_COMMAND".to_string()),
        data: row.try_get("data").unwrap_or(None),
        server_id: row.try_get("server_id").unwrap_or(None),
        channel_id: row.try_get("channel_id").unwrap_or(None),
        user_id: row.try_get("user_id").unwrap(),
        token: row.try_get("token").unwrap(),
        status: row.try_get("status").unwrap_or_else(|_| "pending".to_string()),
        created_at: row.try_get("created_at").unwrap(),
        expires_at: row.try_get("expires_at").unwrap(),
    }
}

// ============================================================================
// Slash Commands
// ============================================================================

pub async fn get_command(pool: &PgPool, command_id: Uuid) -> Result<Option<SlashCommand>> {
    let row = sqlx::query("SELECT * FROM slash_commands WHERE id = $1")
        .bind(command_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_command))
}

pub async fn get_global_commands(pool: &PgPool, application_id: Uuid) -> Result<Vec<SlashCommand>> {
    let rows = sqlx::query(
        "SELECT * FROM slash_commands WHERE application_id = $1 AND server_id IS NULL AND enabled = true ORDER BY name",
    )
    .bind(application_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_command).collect())
}

pub async fn get_server_commands(
    pool: &PgPool,
    application_id: Uuid,
    server_id: Uuid,
) -> Result<Vec<SlashCommand>> {
    let rows = sqlx::query(
        "SELECT * FROM slash_commands WHERE application_id = $1 AND server_id = $2 AND enabled = true ORDER BY name",
    )
    .bind(application_id)
    .bind(server_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_command).collect())
}

pub async fn get_all_server_commands(pool: &PgPool, server_id: Uuid) -> Result<Vec<SlashCommand>> {
    let rows = sqlx::query(
        "SELECT * FROM slash_commands WHERE server_id = $1 AND enabled = true ORDER BY name",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_command).collect())
}

pub async fn upsert_command(
    pool: &PgPool,
    id: Uuid,
    application_id: Uuid,
    server_id: Option<Uuid>,
    name: &str,
    description: &str,
    options: &[CommandOption],
    command_type: i32,
    default_member_permissions: Option<&str>,
    dm_permission: bool,
) -> Result<SlashCommand> {
    let opts = serde_json::to_value(options)?;
    let row = sqlx::query(
        r#"INSERT INTO slash_commands
               (id, application_id, server_id, name, description, options,
                command_type, default_member_permissions, dm_permission)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           ON CONFLICT (application_id, name)
           DO UPDATE SET
               description = EXCLUDED.description,
               options     = EXCLUDED.options,
               command_type = EXCLUDED.command_type,
               default_member_permissions = EXCLUDED.default_member_permissions,
               dm_permission = EXCLUDED.dm_permission,
               updated_at  = NOW()
           RETURNING *"#,
    )
    .bind(id)
    .bind(application_id)
    .bind(server_id)
    .bind(name)
    .bind(description)
    .bind(opts)
    .bind(command_type)
    .bind(default_member_permissions)
    .bind(dm_permission)
    .fetch_one(pool)
    .await?;
    Ok(row_to_command(&row))
}

pub async fn delete_command(pool: &PgPool, command_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM slash_commands WHERE id = $1")
        .bind(command_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn bulk_overwrite_global_commands(
    pool: &PgPool,
    application_id: Uuid,
    commands: &[(Uuid, String, String, serde_json::Value, i32)],
) -> Result<Vec<SlashCommand>> {
    sqlx::query("DELETE FROM slash_commands WHERE application_id = $1 AND server_id IS NULL")
        .bind(application_id)
        .execute(pool)
        .await?;

    let mut result = Vec::new();
    for (id, name, description, options, cmd_type) in commands {
        let row = sqlx::query(
            r#"INSERT INTO slash_commands
                   (id, application_id, name, description, options, command_type)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING *"#,
        )
        .bind(id)
        .bind(application_id)
        .bind(name)
        .bind(description)
        .bind(options)
        .bind(cmd_type)
        .fetch_one(pool)
        .await?;
        result.push(row_to_command(&row));
    }
    Ok(result)
}

// ============================================================================
// Interactions
// ============================================================================

pub async fn create_interaction(
    pool: &PgPool,
    id: Uuid,
    application_id: Uuid,
    interaction_type: &str,
    data: Option<serde_json::Value>,
    server_id: Option<Uuid>,
    channel_id: Option<Uuid>,
    user_id: Uuid,
    token: &str,
) -> Result<Interaction> {
    let row = sqlx::query(
        r#"INSERT INTO interactions
               (id, application_id, interaction_type, data, server_id, channel_id, user_id, token)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING *"#,
    )
    .bind(id)
    .bind(application_id)
    .bind(interaction_type)
    .bind(data)
    .bind(server_id)
    .bind(channel_id)
    .bind(user_id)
    .bind(token)
    .fetch_one(pool)
    .await?;
    Ok(row_to_interaction(&row))
}

pub async fn get_interaction(pool: &PgPool, interaction_id: Uuid) -> Result<Option<Interaction>> {
    let row = sqlx::query("SELECT * FROM interactions WHERE id = $1")
        .bind(interaction_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_interaction))
}

/// Bulk overwrite all commands for a given application in a specific server.
/// Deletes existing server commands for that app, then inserts the new set.
pub async fn bulk_overwrite_server_commands(
    pool: &PgPool,
    application_id: Uuid,
    server_id: Uuid,
    commands: &[(Uuid, String, String, serde_json::Value, i32)],
) -> Result<Vec<SlashCommand>> {
    sqlx::query(
        "DELETE FROM slash_commands WHERE application_id = $1 AND server_id = $2",
    )
    .bind(application_id)
    .bind(server_id)
    .execute(pool)
    .await?;

    let mut result = Vec::new();
    for (id, name, description, options, cmd_type) in commands {
        let row = sqlx::query(
            r#"INSERT INTO slash_commands
                   (id, application_id, server_id, name, description, options, command_type)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING *"#,
        )
        .bind(id)
        .bind(application_id)
        .bind(server_id)
        .bind(name)
        .bind(description)
        .bind(options)
        .bind(cmd_type)
        .fetch_one(pool)
        .await?;
        result.push(row_to_command(&row));
    }
    Ok(result)
}
