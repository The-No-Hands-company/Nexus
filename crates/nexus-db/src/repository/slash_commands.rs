//! Slash command and interaction repository.

use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;

use nexus_common::models::slash_command::{CommandOption, Interaction, SlashCommand};

fn row_to_command(row: &sqlx::any::AnyRow) -> SlashCommand {
    let options: Vec<CommandOption> = row.try_get::<Option<String>, _>("options").unwrap_or(None)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    SlashCommand {
        id: row.try_get::<String, _>("id").unwrap_or_default().parse().unwrap_or_default(),
        application_id: row.try_get::<String, _>("application_id").unwrap_or_default().parse().unwrap_or_default(),
        server_id: row.try_get::<Option<String>, _>("server_id").unwrap_or(None).and_then(|s| s.parse().ok()),
        name: row.try_get("name").unwrap_or_default(),
        name_localizations: row.try_get::<Option<String>, _>("name_localizations").unwrap_or(None)
            .and_then(|s| serde_json::from_str(&s).ok()),
        description: row.try_get("description").unwrap_or_default(),
        description_localizations: row.try_get::<Option<String>, _>("description_localizations").unwrap_or(None)
            .and_then(|s| serde_json::from_str(&s).ok()),
        options,
        default_member_permissions: row.try_get("default_member_permissions").unwrap_or(None),
        dm_permission: row.try_get("dm_permission").unwrap_or(true),
        command_type: row.try_get("command_type").unwrap_or(1),
        version: row.try_get::<String, _>("version").unwrap_or_default().parse().unwrap_or_default(),
        enabled: row.try_get("enabled").unwrap_or(true),
        created_at: crate::any_compat::get_datetime(row, "created_at").unwrap_or_default(),
        updated_at: crate::any_compat::get_datetime(row, "updated_at").unwrap_or_default(),
    }
}

fn row_to_interaction(row: &sqlx::any::AnyRow) -> Interaction {
    Interaction {
        id: row.try_get::<String, _>("id").unwrap_or_default().parse().unwrap_or_default(),
        application_id: row.try_get::<String, _>("application_id").unwrap_or_default().parse().unwrap_or_default(),
        interaction_type: row.try_get("interaction_type").unwrap_or_else(|_| "APPLICATION_COMMAND".to_string()),
        data: row.try_get::<Option<String>, _>("data").unwrap_or(None).and_then(|s| serde_json::from_str(&s).ok()),
        server_id: row.try_get::<Option<String>, _>("server_id").unwrap_or(None).and_then(|s| s.parse().ok()),
        channel_id: row.try_get::<Option<String>, _>("channel_id").unwrap_or(None).and_then(|s| s.parse().ok()),
        user_id: row.try_get::<String, _>("user_id").unwrap_or_default().parse().unwrap_or_default(),
        token: row.try_get("token").unwrap_or_default(),
        status: row.try_get("status").unwrap_or_else(|_| "pending".to_string()),
        created_at: crate::any_compat::get_datetime(row, "created_at").unwrap_or_default(),
        expires_at: crate::any_compat::get_datetime(row, "expires_at").unwrap_or_default(),
    }
}

// ============================================================================
// Slash Commands
// ============================================================================

pub async fn get_command(pool: &sqlx::AnyPool, command_id: Uuid) -> Result<Option<SlashCommand>> {
    let row = sqlx::query("SELECT * FROM slash_commands WHERE id = ?")
        .bind(command_id.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_command))
}

pub async fn get_global_commands(pool: &sqlx::AnyPool, application_id: Uuid) -> Result<Vec<SlashCommand>> {
    let rows = sqlx::query(
        "SELECT * FROM slash_commands WHERE application_id = ? AND server_id IS NULL AND enabled = true ORDER BY name",
    )
    .bind(application_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_command).collect())
}

pub async fn get_server_commands(
    pool: &sqlx::AnyPool,
    application_id: Uuid,
    server_id: Uuid,
) -> Result<Vec<SlashCommand>> {
    let rows = sqlx::query(
        "SELECT * FROM slash_commands WHERE application_id = ? AND server_id = ? AND enabled = true ORDER BY name",
    )
    .bind(application_id.to_string())
    .bind(server_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_command).collect())
}

pub async fn get_all_server_commands(pool: &sqlx::AnyPool, server_id: Uuid) -> Result<Vec<SlashCommand>> {
    let rows = sqlx::query(
        "SELECT * FROM slash_commands WHERE server_id = ? AND enabled = true ORDER BY name",
    )
    .bind(server_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_command).collect())
}

pub async fn upsert_command(
    pool: &sqlx::AnyPool,
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
    let opts = serde_json::to_string(options)?;
    let row = sqlx::query(
        r#"INSERT INTO slash_commands
               (id, application_id, server_id, name, description, options,
                command_type, default_member_permissions, dm_permission)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT (application_id, name)
           DO UPDATE SET
               description = EXCLUDED.description,
               options     = EXCLUDED.options,
               command_type = EXCLUDED.command_type,
               default_member_permissions = EXCLUDED.default_member_permissions,
               dm_permission = EXCLUDED.dm_permission,
               updated_at  = CURRENT_TIMESTAMP
           RETURNING *"#,
    )
    .bind(id.to_string())
    .bind(application_id.to_string())
    .bind(server_id.map(|u| u.to_string()))
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

pub async fn delete_command(pool: &sqlx::AnyPool, command_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM slash_commands WHERE id = ?")
        .bind(command_id.to_string())
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn bulk_overwrite_global_commands(
    pool: &sqlx::AnyPool,
    application_id: Uuid,
    commands: &[(Uuid, String, String, serde_json::Value, i32)],
) -> Result<Vec<SlashCommand>> {
    sqlx::query("DELETE FROM slash_commands WHERE application_id = ? AND server_id IS NULL")
        .bind(application_id.to_string())
        .execute(pool)
        .await?;

    let mut result = Vec::new();
    for (id, name, description, options, cmd_type) in commands {
        let row = sqlx::query(
            r#"INSERT INTO slash_commands
                   (id, application_id, name, description, options, command_type)
               VALUES (?, ?, ?, ?, ?, ?)
               RETURNING *"#,
        )
        .bind(id.to_string())
        .bind(application_id.to_string())
        .bind(name)
        .bind(description)
        .bind(serde_json::to_string(options).unwrap_or_default())
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
    pool: &sqlx::AnyPool,
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
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)
           RETURNING *"#,
    )
    .bind(id.to_string())
    .bind(application_id.to_string())
    .bind(interaction_type)
    .bind(data.as_ref().map(|v| serde_json::to_string(v).unwrap_or_default()))
    .bind(server_id.map(|u| u.to_string()))
    .bind(channel_id.map(|u| u.to_string()))
    .bind(user_id.to_string())
    .bind(token)
    .fetch_one(pool)
    .await?;
    Ok(row_to_interaction(&row))
}

pub async fn get_interaction(pool: &sqlx::AnyPool, interaction_id: Uuid) -> Result<Option<Interaction>> {
    let row = sqlx::query("SELECT * FROM interactions WHERE id = ?")
        .bind(interaction_id.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_interaction))
}

/// Bulk overwrite all commands for a given application in a specific server.
/// Deletes existing server commands for that app, then inserts the new set.
pub async fn bulk_overwrite_server_commands(
    pool: &sqlx::AnyPool,
    application_id: Uuid,
    server_id: Uuid,
    commands: &[(Uuid, String, String, serde_json::Value, i32)],
) -> Result<Vec<SlashCommand>> {
    sqlx::query(
        "DELETE FROM slash_commands WHERE application_id = ? AND server_id = ?",
    )
    .bind(application_id.to_string())
    .bind(server_id.to_string())
    .execute(pool)
    .await?;

    let mut result = Vec::new();
    for (id, name, description, options, cmd_type) in commands {
        let row = sqlx::query(
            r#"INSERT INTO slash_commands
                   (id, application_id, server_id, name, description, options, command_type)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               RETURNING *"#,
        )
        .bind(id.to_string())
        .bind(application_id.to_string())
        .bind(server_id.to_string())
        .bind(name)
        .bind(description)
        .bind(serde_json::to_string(options).unwrap_or_default())
        .bind(cmd_type)
        .fetch_one(pool)
        .await?;
        result.push(row_to_command(&row));
    }
    Ok(result)
}
