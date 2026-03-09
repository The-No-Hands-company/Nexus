//! Repository for server analytics snapshots.

use nexus_common::models::ecosystem::ServerAnalyticsSnapshot;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::SERVER_ANALYTICS_SNAPSHOT_COLS;

pub async fn upsert_snapshot(
    pool: &AnyPool,
    id: Uuid,
    server_id: Uuid,
    period_date: &str,
    messages_count: i32,
    active_members: i32,
    new_members: i32,
    left_members: i32,
    voice_minutes: i32,
    reports_resolved: i32,
    bans_issued: i32,
    filters_triggered: i32,
) -> Result<ServerAnalyticsSnapshot, sqlx::Error> {
    let q = format!(
        "INSERT INTO server_analytics_snapshots \
         (id, server_id, period_date, messages_count, active_members, new_members, \
          left_members, voice_minutes, reports_resolved, bans_issued, filters_triggered) \
         VALUES ($1, $2, $3::date, $4, $5, $6, $7, $8, $9, $10, $11) \
         ON CONFLICT (server_id, period_date) DO UPDATE SET \
           messages_count = $4, active_members = $5, new_members = $6, \
           left_members = $7, voice_minutes = $8, reports_resolved = $9, \
           bans_issued = $10, filters_triggered = $11 \
         RETURNING {SERVER_ANALYTICS_SNAPSHOT_COLS}"
    );
    sqlx::query_as::<_, ServerAnalyticsSnapshot>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(period_date)
        .bind(messages_count)
        .bind(active_members)
        .bind(new_members)
        .bind(left_members)
        .bind(voice_minutes)
        .bind(reports_resolved)
        .bind(bans_issued)
        .bind(filters_triggered)
        .fetch_one(pool)
        .await
}

pub async fn list_snapshots(
    pool: &AnyPool,
    server_id: Uuid,
    days: i64,
) -> Result<Vec<ServerAnalyticsSnapshot>, sqlx::Error> {
    let q = format!(
        "SELECT {SERVER_ANALYTICS_SNAPSHOT_COLS} FROM server_analytics_snapshots \
         WHERE server_id = $1 AND period_date >= (CURRENT_DATE - $2 * INTERVAL '1 day')::date \
         ORDER BY period_date ASC"
    );
    sqlx::query_as::<_, ServerAnalyticsSnapshot>(&q)
        .bind(server_id.to_string())
        .bind(days)
        .fetch_all(pool)
        .await
}
