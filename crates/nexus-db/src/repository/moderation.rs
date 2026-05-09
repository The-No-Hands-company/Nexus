//! Moderation repository — timeouts, bans (with expiry), message reports, word filters.
//!
//! All tables managed here were created in migration 00010.

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::any_compat::{get_datetime, get_opt_datetime, get_opt_uuid, get_uuid};

// ── Timeouts ─────────────────────────────────────────────────────────────────

/// Set (or replace) a communication timeout for a member.
///
/// Both the `user_timeouts` metadata table **and** the
/// `members.communication_disabled_until` column are updated so that
/// enforcement checks are a single column read.
pub async fn set_timeout(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
    moderator_id: Uuid,
    expires_at: DateTime<Utc>,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    // Upsert the timeout metadata record.
    sqlx::query(
        "INSERT INTO user_timeouts \
         (user_id, server_id, moderator_id, expires_at, reason, created_at) \
         VALUES ($1::uuid, $2::uuid, $3::uuid, $4, $5, NOW()) \
         ON CONFLICT (user_id, server_id) DO UPDATE \
             SET moderator_id = EXCLUDED.moderator_id, \
                 expires_at   = EXCLUDED.expires_at, \
                 reason       = EXCLUDED.reason, \
                 created_at   = NOW()",
    )
    .bind(user_id.to_string())
    .bind(server_id.to_string())
    .bind(moderator_id.to_string())
    .bind(expires_at.to_rfc3339())
    .bind(reason)
    .execute(pool)
    .await?;

    // Mirror to the members column for efficient inline enforcement.
    sqlx::query(
        "UPDATE members \
         SET communication_disabled_until = $1 \
         WHERE user_id = $2::uuid AND server_id = $3::uuid",
    )
    .bind(expires_at.to_rfc3339())
    .bind(user_id.to_string())
    .bind(server_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

/// Return the timeout expiry if one is currently active for the member, else `None`.
pub async fn get_active_timeout(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT expires_at::text \
         FROM user_timeouts \
         WHERE user_id = $1::uuid AND server_id = $2::uuid AND expires_at > NOW()",
    )
    .bind(user_id.to_string())
    .bind(server_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|(s,)| s.parse::<DateTime<Utc>>().ok()))
}

/// Lift an active timeout early (e.g. moderator removes it manually).
///
/// Clears both the `user_timeouts` row and the `members` column.
/// Returns `true` if a record was actually removed.
pub async fn lift_timeout(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM user_timeouts WHERE user_id = $1::uuid AND server_id = $2::uuid")
            .bind(user_id.to_string())
            .bind(server_id.to_string())
            .execute(pool)
            .await?;

    // Always clear the members column, even if no timeout row existed.
    sqlx::query(
        "UPDATE members \
         SET communication_disabled_until = NULL \
         WHERE user_id = $1::uuid AND server_id = $2::uuid",
    )
    .bind(user_id.to_string())
    .bind(server_id.to_string())
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Purge all expired timeout rows and clear the corresponding member columns.
///
/// Intended to be called periodically by a background task.
/// Returns the number of timeout rows removed.
pub async fn purge_expired_timeouts(pool: &sqlx::AnyPool) -> Result<u64, sqlx::Error> {
    // Clear members first so the UI sees the lift promptly.
    sqlx::query(
        "UPDATE members \
         SET communication_disabled_until = NULL \
         WHERE communication_disabled_until IS NOT NULL \
           AND communication_disabled_until <= NOW()",
    )
    .execute(pool)
    .await?;

    let result = sqlx::query("DELETE FROM user_timeouts WHERE expires_at <= NOW()")
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

// ── Bans ─────────────────────────────────────────────────────────────────────

/// A ban record.
#[derive(Debug, Serialize)]
pub struct BanRow {
    pub user_id: Uuid,
    pub server_id: Uuid,
    pub reason: Option<String>,
    /// The user who issued the ban.
    pub banned_by: Uuid,
    /// `None` = permanent ban.
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> for BanRow {
    fn from_row(row: &'r sqlx::any::AnyRow) -> Result<Self, sqlx::Error> {
        Ok(BanRow {
            user_id: get_uuid(row, "user_id")?,
            server_id: get_uuid(row, "server_id")?,
            reason: sqlx::Row::try_get(row, "reason").ok().flatten(),
            banned_by: get_uuid(row, "banned_by")?,
            expires_at: get_opt_datetime(row, "expires_at")?,
            created_at: get_datetime(row, "created_at")?,
        })
    }
}

const BAN_COLS: &str = "user_id::text    AS user_id, \
                         server_id::text  AS server_id, \
                         reason, \
                         banned_by::text  AS banned_by, \
                         expires_at::text AS expires_at, \
                         created_at::text AS created_at";

/// Create or replace a ban.
///
/// `expires_at = None` creates a permanent ban.
/// `expires_at = Some(t)` creates a temp-ban that can be lifted by the purge task.
pub async fn create_ban(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
    banned_by: Uuid,
    reason: Option<&str>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<BanRow, sqlx::Error> {
    let q = format!(
        "INSERT INTO bans \
         (user_id, server_id, banned_by, reason, expires_at, created_at) \
         VALUES ($1::uuid, $2::uuid, $3::uuid, $4, $5, NOW()) \
         ON CONFLICT (user_id, server_id) DO UPDATE \
             SET banned_by  = EXCLUDED.banned_by, \
                 reason     = EXCLUDED.reason, \
                 expires_at = EXCLUDED.expires_at, \
                 created_at = NOW() \
         RETURNING {BAN_COLS}"
    );
    sqlx::query_as::<_, BanRow>(&q)
        .bind(user_id.to_string())
        .bind(server_id.to_string())
        .bind(banned_by.to_string())
        .bind(reason)
        .bind(expires_at.map(|t| t.to_rfc3339()))
        .fetch_one(pool)
        .await
}

/// Look up a single ban entry (permanent or temporary, including expired).
pub async fn get_ban(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<Option<BanRow>, sqlx::Error> {
    let q = format!(
        "SELECT {BAN_COLS} FROM bans \
         WHERE user_id = $1::uuid AND server_id = $2::uuid"
    );
    sqlx::query_as::<_, BanRow>(&q)
        .bind(user_id.to_string())
        .bind(server_id.to_string())
        .fetch_optional(pool)
        .await
}

/// Check whether a user is currently (non-expired) banned from a server.
pub async fn is_banned(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM bans \
         WHERE user_id   = $1::uuid \
           AND server_id = $2::uuid \
           AND (expires_at IS NULL OR expires_at > NOW())",
    )
    .bind(user_id.to_string())
    .bind(server_id.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

/// List all active (non-expired) bans for a server, newest first.
pub async fn list_bans(pool: &sqlx::AnyPool, server_id: Uuid) -> Result<Vec<BanRow>, sqlx::Error> {
    let q = format!(
        "SELECT {BAN_COLS} FROM bans \
         WHERE server_id = $1::uuid \
           AND (expires_at IS NULL OR expires_at > NOW()) \
         ORDER BY created_at DESC"
    );
    sqlx::query_as::<_, BanRow>(&q)
        .bind(server_id.to_string())
        .fetch_all(pool)
        .await
}

/// Remove a ban entry.  Returns `true` if one was actually deleted.
pub async fn remove_ban(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM bans WHERE user_id = $1::uuid AND server_id = $2::uuid")
        .bind(user_id.to_string())
        .bind(server_id.to_string())
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Purge all expired temp-ban rows.
///
/// Intended to be called by a periodic background task.
/// Returns the number of rows removed.
pub async fn purge_expired_bans(pool: &sqlx::AnyPool) -> Result<u64, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM bans WHERE expires_at IS NOT NULL AND expires_at <= NOW()")
            .execute(pool)
            .await?;
    Ok(result.rows_affected())
}

// ── Message reports ───────────────────────────────────────────────────────────

/// A single message report as stored in the database.
#[derive(Debug, Serialize)]
pub struct ReportRow {
    pub id: Uuid,
    pub message_id: Uuid,
    pub channel_id: Uuid,
    pub server_id: Option<Uuid>,
    pub reporter_id: Uuid,
    pub reason: String,
    /// "pending" | "resolved" | "dismissed"
    pub status: String,
    pub resolved_by: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_action: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> for ReportRow {
    fn from_row(row: &'r sqlx::any::AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ReportRow {
            id: get_uuid(row, "id")?,
            message_id: get_uuid(row, "message_id")?,
            channel_id: get_uuid(row, "channel_id")?,
            server_id: get_opt_uuid(row, "server_id")?,
            reporter_id: get_uuid(row, "reporter_id")?,
            reason: sqlx::Row::try_get(row, "reason")?,
            status: sqlx::Row::try_get(row, "status")?,
            resolved_by: get_opt_uuid(row, "resolved_by")?,
            resolved_at: get_opt_datetime(row, "resolved_at")?,
            resolution_action: sqlx::Row::try_get(row, "resolution_action").ok().flatten(),
            created_at: get_datetime(row, "created_at")?,
        })
    }
}

const REPORT_COLS: &str = "id::text          AS id, \
     message_id::text  AS message_id, \
     channel_id::text  AS channel_id, \
     server_id::text   AS server_id, \
     reporter_id::text AS reporter_id, \
     reason, status, \
     resolved_by::text AS resolved_by, \
     resolved_at::text AS resolved_at, \
     resolution_action, \
     created_at::text  AS created_at";

/// File a new message report.
///
/// The database-level deduplication index prevents the same user from
/// filing more than one *pending* report for the same message.
pub async fn create_report(
    pool: &sqlx::AnyPool,
    id: Uuid,
    message_id: Uuid,
    channel_id: Uuid,
    server_id: Option<Uuid>,
    reporter_id: Uuid,
    reason: &str,
) -> Result<ReportRow, sqlx::Error> {
    let q = format!(
        "INSERT INTO message_reports \
         (id, message_id, channel_id, server_id, reporter_id, reason, status, created_at) \
         VALUES ($1::uuid, $2::uuid, $3::uuid, $4::uuid, $5::uuid, $6, 'pending', NOW()) \
         RETURNING {REPORT_COLS}"
    );
    sqlx::query_as::<_, ReportRow>(&q)
        .bind(id.to_string())
        .bind(message_id.to_string())
        .bind(channel_id.to_string())
        .bind(server_id.map(|u| u.to_string()))
        .bind(reporter_id.to_string())
        .bind(reason)
        .fetch_one(pool)
        .await
}

/// List reports for a server, newest first (max 100).
///
/// `status = None` returns all statuses; otherwise filters to the given value.
pub async fn list_reports(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
    status: Option<&str>,
) -> Result<Vec<ReportRow>, sqlx::Error> {
    let (q, status_str) = if let Some(s) = status {
        (
            format!(
                "SELECT {REPORT_COLS} FROM message_reports \
                 WHERE server_id = $1::uuid AND status = $2 \
                 ORDER BY created_at DESC LIMIT 100"
            ),
            Some(s.to_string()),
        )
    } else {
        (
            format!(
                "SELECT {REPORT_COLS} FROM message_reports \
                 WHERE server_id = $1::uuid \
                 ORDER BY created_at DESC LIMIT 100"
            ),
            None,
        )
    };

    let mut query = sqlx::query_as::<_, ReportRow>(&q).bind(server_id.to_string());
    if let Some(s) = status_str {
        query = query.bind(s);
    }
    query.fetch_all(pool).await
}

/// Mark a pending report as resolved with an action description.
///
/// Returns `true` if the row was found and updated, `false` if it was already
/// resolved/dismissed or did not belong to the given server.
pub async fn resolve_report(
    pool: &sqlx::AnyPool,
    report_id: Uuid,
    server_id: Uuid,
    resolver_id: Uuid,
    action: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE message_reports \
         SET status = 'resolved', \
             resolved_by = $1::uuid, \
             resolved_at = NOW(), \
             resolution_action = $2 \
         WHERE id = $3::uuid \
           AND server_id = $4::uuid \
           AND status = 'pending'",
    )
    .bind(resolver_id.to_string())
    .bind(action)
    .bind(report_id.to_string())
    .bind(server_id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Mark a pending report as dismissed (no action taken).
pub async fn dismiss_report(
    pool: &sqlx::AnyPool,
    report_id: Uuid,
    server_id: Uuid,
    resolver_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE message_reports \
         SET status = 'dismissed', \
             resolved_by = $1::uuid, \
             resolved_at = NOW() \
         WHERE id = $2::uuid \
           AND server_id = $3::uuid \
           AND status = 'pending'",
    )
    .bind(resolver_id.to_string())
    .bind(report_id.to_string())
    .bind(server_id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// ── Word filters ──────────────────────────────────────────────────────────────

/// A server word/phrase filter rule.
#[derive(Debug, Serialize)]
pub struct WordFilter {
    pub id: Uuid,
    pub server_id: Uuid,
    /// The literal pattern to match (case-insensitive substring match).
    pub pattern: String,
    /// "block" | "delete" | "warn"
    pub action: String,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> for WordFilter {
    fn from_row(row: &'r sqlx::any::AnyRow) -> Result<Self, sqlx::Error> {
        Ok(WordFilter {
            id: get_uuid(row, "id")?,
            server_id: get_uuid(row, "server_id")?,
            pattern: sqlx::Row::try_get(row, "pattern")?,
            action: sqlx::Row::try_get(row, "action")?,
            created_by: get_opt_uuid(row, "created_by")?,
            created_at: get_datetime(row, "created_at")?,
        })
    }
}

const FILTER_COLS: &str = "id::text         AS id, \
     server_id::text  AS server_id, \
     pattern, action, \
     created_by::text AS created_by, \
     created_at::text AS created_at";

/// Add a word filter rule for a server.
///
/// Duplicate patterns (same server + pattern) are rejected by the DB unique index.
pub async fn add_filter(
    pool: &sqlx::AnyPool,
    id: Uuid,
    server_id: Uuid,
    pattern: &str,
    action: &str,
    created_by: Uuid,
) -> Result<WordFilter, sqlx::Error> {
    let q = format!(
        "INSERT INTO server_word_filters \
         (id, server_id, pattern, action, created_by, created_at) \
         VALUES ($1::uuid, $2::uuid, $3, $4, $5::uuid, NOW()) \
         RETURNING {FILTER_COLS}"
    );
    sqlx::query_as::<_, WordFilter>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(pattern)
        .bind(action)
        .bind(created_by.to_string())
        .fetch_one(pool)
        .await
}

/// List all word filter rules for a server (oldest first for stable ordering).
pub async fn list_filters(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
) -> Result<Vec<WordFilter>, sqlx::Error> {
    let q = format!(
        "SELECT {FILTER_COLS} FROM server_word_filters \
         WHERE server_id = $1::uuid \
         ORDER BY created_at ASC"
    );
    sqlx::query_as::<_, WordFilter>(&q)
        .bind(server_id.to_string())
        .fetch_all(pool)
        .await
}

/// Delete a word filter rule.  Returns `true` if it existed.
pub async fn remove_filter(
    pool: &sqlx::AnyPool,
    filter_id: Uuid,
    server_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM server_word_filters \
         WHERE id = $1::uuid AND server_id = $2::uuid",
    )
    .bind(filter_id.to_string())
    .bind(server_id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Check message content against all word filter rules for a server.
///
/// Returns `Some((matched_pattern, action))` on the first match, or `None` if
/// the content is clean.  Matching is case-insensitive substring search.
pub async fn check_content(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
    content: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let filters = list_filters(pool, server_id).await?;
    let lower = content.to_lowercase();
    for filter in filters {
        if lower.contains(&filter.pattern.to_lowercase()) {
            return Ok(Some((filter.pattern, filter.action)));
        }
    }
    Ok(None)
}
