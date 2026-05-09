//! Repository: member_prune_rules + slow mode overrides — Phase 20-04.

use nexus_common::models::scalability::{MemberPruneRule, SlowModeOverride};
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::{MEMBER_PRUNE_RULE_COLS, SLOW_MODE_OVERRIDE_COLS};

// ── Member Prune Rules ────────────────────────────────────────────────────

pub async fn upsert_prune_rule(
    pool: &AnyPool,
    id: Uuid,
    server_id: Uuid,
    inactivity_days: i32,
    grace_period_days: i32,
    exclude_roles: &serde_json::Value,
    notify_before: bool,
    is_enabled: bool,
) -> Result<MemberPruneRule, sqlx::Error> {
    let q = format!(
        "INSERT INTO member_prune_rules \
         (id, server_id, inactivity_days, grace_period_days, exclude_roles, notify_before, is_enabled) \
         VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7) \
         ON CONFLICT (server_id) DO UPDATE SET \
           inactivity_days = $3, grace_period_days = $4, exclude_roles = $5::jsonb, \
           notify_before = $6, is_enabled = $7, updated_at = now() \
         RETURNING {MEMBER_PRUNE_RULE_COLS}"
    );
    sqlx::query_as::<_, MemberPruneRule>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(inactivity_days)
        .bind(grace_period_days)
        .bind(exclude_roles.to_string())
        .bind(notify_before)
        .bind(is_enabled)
        .fetch_one(pool)
        .await
}

pub async fn get_prune_rule(
    pool: &AnyPool,
    server_id: Uuid,
) -> Result<Option<MemberPruneRule>, sqlx::Error> {
    let q = format!("SELECT {MEMBER_PRUNE_RULE_COLS} FROM member_prune_rules WHERE server_id = $1");
    sqlx::query_as::<_, MemberPruneRule>(&q)
        .bind(server_id.to_string())
        .fetch_optional(pool)
        .await
}

// ── Slow Mode Overrides ───────────────────────────────────────────────────

pub async fn upsert_slow_mode_override(
    pool: &AnyPool,
    id: Uuid,
    channel_id: Uuid,
    role_id: Option<Uuid>,
    cooldown_secs: i32,
    escalation_mult: f32,
) -> Result<SlowModeOverride, sqlx::Error> {
    let q = format!(
        "INSERT INTO slow_mode_overrides (id, channel_id, role_id, cooldown_secs, escalation_mult) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (channel_id, role_id) DO UPDATE SET \
           cooldown_secs = $4, escalation_mult = $5 \
         RETURNING {SLOW_MODE_OVERRIDE_COLS}"
    );
    sqlx::query_as::<_, SlowModeOverride>(&q)
        .bind(id.to_string())
        .bind(channel_id.to_string())
        .bind(role_id.map(|u| u.to_string()))
        .bind(cooldown_secs)
        .bind(escalation_mult as f64)
        .fetch_one(pool)
        .await
}

pub async fn list_slow_mode_overrides(
    pool: &AnyPool,
    channel_id: Uuid,
) -> Result<Vec<SlowModeOverride>, sqlx::Error> {
    let q =
        format!("SELECT {SLOW_MODE_OVERRIDE_COLS} FROM slow_mode_overrides WHERE channel_id = $1");
    sqlx::query_as::<_, SlowModeOverride>(&q)
        .bind(channel_id.to_string())
        .fetch_all(pool)
        .await
}
