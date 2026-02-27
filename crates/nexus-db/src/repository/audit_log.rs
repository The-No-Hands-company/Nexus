//! Audit log repository — structured record of moderation actions.
//!
//! Uses the existing `audit_log` table (created in migration 00001, corrected
//! in 00010 to make `user_id` nullable so system actions can be recorded).

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::any_compat::{get_datetime, get_opt_uuid, get_uuid};

/// A single audit log entry returned by list queries.
#[derive(Debug, Serialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub server_id: Uuid,
    /// The user who performed the action (`None` = automated/system action).
    pub actor_id: Option<Uuid>,
    /// Machine-readable action code: "MEMBER_KICK", "MEMBER_BAN", etc.
    pub action: String,
    /// What kind of entity was affected: "member", "channel", "role", …
    pub target_type: Option<String>,
    /// ID of the affected entity.
    pub target_id: Option<Uuid>,
    /// Free-form JSON diff / metadata for the action.
    pub changes: serde_json::Value,
    /// Optional human-readable reason provided by the moderator.
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> for AuditLogEntry {
    fn from_row(row: &'r sqlx::any::AnyRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        let changes_str: Option<String> = row.try_get("changes").ok().flatten();
        let changes = changes_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));

        Ok(AuditLogEntry {
            id: get_uuid(row, "id")?,
            server_id: get_uuid(row, "server_id")?,
            actor_id: get_opt_uuid(row, "user_id")?,
            action: row.try_get("action")?,
            target_type: row.try_get("target_type").ok().flatten(),
            target_id: get_opt_uuid(row, "target_id")?,
            changes,
            reason: row.try_get("reason").ok().flatten(),
            created_at: get_datetime(row, "created_at")?,
        })
    }
}

/// Write a new audit log entry.
///
/// `actor_id = None` is valid for automated/system entries.
#[allow(clippy::too_many_arguments)]
pub async fn write_entry(
    pool: &sqlx::AnyPool,
    id: Uuid,
    server_id: Uuid,
    actor_id: Option<Uuid>,
    action: &str,
    target_type: Option<&str>,
    target_id: Option<Uuid>,
    changes: &serde_json::Value,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_log \
         (id, server_id, user_id, action, target_type, target_id, changes, reason, created_at) \
         VALUES ($1::uuid, $2::uuid, $3::uuid, $4, $5, $6::uuid, $7::jsonb, $8, NOW())",
    )
    .bind(id.to_string())
    .bind(server_id.to_string())
    .bind(actor_id.map(|u| u.to_string()))
    .bind(action)
    .bind(target_type)
    .bind(target_id.map(|u| u.to_string()))
    .bind(serde_json::to_string(changes).unwrap_or_else(|_| "{}".to_string()))
    .bind(reason)
    .execute(pool)
    .await?;
    Ok(())
}

/// List audit log entries for a server, newest first.
///
/// Optional filters:
/// - `action_filter`: exact match on `action` column (e.g. `"MEMBER_KICK"`)
/// - `actor_filter`:  only entries from the given user ID
/// - `limit`: maximum rows to return (capped at 100 by callers)
pub async fn list_entries(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
    action_filter: Option<&str>,
    actor_filter: Option<Uuid>,
    limit: i64,
) -> Result<Vec<AuditLogEntry>, sqlx::Error> {
    // Build query dynamically (AnyPool does not support named params).
    let mut conditions = vec!["server_id = $1::uuid".to_string()];
    let mut string_args: Vec<String> = vec![server_id.to_string()];
    let mut param_idx: usize = 2;

    if let Some(action) = action_filter {
        conditions.push(format!("action = ${param_idx}"));
        string_args.push(action.to_string());
        param_idx += 1;
    }
    if let Some(actor) = actor_filter {
        conditions.push(format!("user_id = ${}::uuid", param_idx));
        string_args.push(actor.to_string());
        param_idx += 1;
    }

    let where_clause = conditions.join(" AND ");
    let q = format!(
        "SELECT \
            id::text          AS id, \
            server_id::text   AS server_id, \
            user_id::text     AS user_id, \
            action, \
            target_type, \
            target_id::text   AS target_id, \
            COALESCE(changes::text, '{{}}') AS changes, \
            reason, \
            created_at::text  AS created_at \
         FROM audit_log \
         WHERE {where_clause} \
         ORDER BY created_at DESC \
         LIMIT ${param_idx}"
    );

    let mut query = sqlx::query_as::<_, AuditLogEntry>(&q);
    for arg in &string_args {
        query = query.bind(arg.clone());
    }
    query = query.bind(limit);

    query.fetch_all(pool).await
}
