//! Repository: growth & retention — Phase 23.

use nexus_common::models::growth::*;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::*;

// ── 23-01: Server Recommendations ─────────────────────────────────────────

pub async fn upsert_recommendation(
    pool: &AnyPool,
    id: Uuid,
    user_id: Uuid,
    server_id: Uuid,
    score: f32,
    reason: &str,
) -> Result<ServerRecommendation, sqlx::Error> {
    let q = format!(
        "INSERT INTO server_recommendations (id, user_id, server_id, score, reason) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (user_id, server_id) DO UPDATE SET score = $4, reason = $5 \
         RETURNING {SERVER_RECOMMENDATION_COLS}"
    );
    sqlx::query_as::<_, ServerRecommendation>(&q)
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(server_id.to_string())
        .bind(score as f64)
        .bind(reason)
        .fetch_one(pool)
        .await
}

pub async fn list_recommendations(
    pool: &AnyPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<ServerRecommendation>, sqlx::Error> {
    let q = format!(
        "SELECT {SERVER_RECOMMENDATION_COLS} FROM server_recommendations \
         WHERE user_id = $1 AND dismissed = FALSE AND joined = FALSE \
         ORDER BY score DESC LIMIT $2"
    );
    sqlx::query_as::<_, ServerRecommendation>(&q)
        .bind(user_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn dismiss_recommendation(
    pool: &AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE server_recommendations SET dismissed = TRUE WHERE user_id = $1 AND server_id = $2")
        .bind(user_id.to_string())
        .bind(server_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

// ── Onboarding Flows ──────────────────────────────────────────────────────

pub async fn upsert_onboarding_flow(
    pool: &AnyPool,
    id: Uuid,
    server_id: Uuid,
    steps: &serde_json::Value,
    adaptive: bool,
    skip_completed: bool,
) -> Result<OnboardingFlow, sqlx::Error> {
    let q = format!(
        "INSERT INTO onboarding_flows (id, server_id, steps, adaptive, skip_completed) \
         VALUES ($1, $2, $3::jsonb, $4, $5) \
         ON CONFLICT (server_id) DO UPDATE SET \
           steps = $3::jsonb, adaptive = $4, skip_completed = $5, updated_at = now() \
         RETURNING {ONBOARDING_FLOW_COLS}"
    );
    sqlx::query_as::<_, OnboardingFlow>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(steps.to_string())
        .bind(adaptive)
        .bind(skip_completed)
        .fetch_one(pool)
        .await
}

pub async fn get_onboarding_flow(
    pool: &AnyPool,
    server_id: Uuid,
) -> Result<Option<OnboardingFlow>, sqlx::Error> {
    let q = format!(
        "SELECT {ONBOARDING_FLOW_COLS} FROM onboarding_flows WHERE server_id = $1 AND is_active = TRUE"
    );
    sqlx::query_as::<_, OnboardingFlow>(&q)
        .bind(server_id.to_string())
        .fetch_optional(pool)
        .await
}

// ── 23-02: Device Sessions ────────────────────────────────────────────────

pub async fn upsert_device_session(
    pool: &AnyPool,
    id: Uuid,
    user_id: Uuid,
    device_id: &str,
    device_type: &str,
    last_channel_id: Option<Uuid>,
) -> Result<DeviceSession, sqlx::Error> {
    let q = format!(
        "INSERT INTO device_sessions (id, user_id, device_id, device_type, last_channel_id) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (user_id, device_id) DO UPDATE SET \
           device_type = $4, last_channel_id = $5, last_seen_at = now(), is_active = TRUE \
         RETURNING {DEVICE_SESSION_COLS}"
    );
    sqlx::query_as::<_, DeviceSession>(&q)
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(device_id)
        .bind(device_type)
        .bind(last_channel_id.map(|u| u.to_string()))
        .fetch_one(pool)
        .await
}

pub async fn list_device_sessions(
    pool: &AnyPool,
    user_id: Uuid,
) -> Result<Vec<DeviceSession>, sqlx::Error> {
    let q = format!(
        "SELECT {DEVICE_SESSION_COLS} FROM device_sessions WHERE user_id = $1 AND is_active = TRUE"
    );
    sqlx::query_as::<_, DeviceSession>(&q)
        .bind(user_id.to_string())
        .fetch_all(pool)
        .await
}

// ── 23-03: Gamification ───────────────────────────────────────────────────

pub async fn upsert_gamification_config(
    pool: &AnyPool,
    server_id: Uuid,
    enabled: bool,
    xp_per_message: i32,
    xp_per_reaction: i32,
    xp_per_voice_min: i32,
    level_formula: &str,
    streak_enabled: bool,
) -> Result<GamificationConfig, sqlx::Error> {
    let q = format!(
        "INSERT INTO gamification_configs \
         (server_id, enabled, xp_per_message, xp_per_reaction, xp_per_voice_min, level_formula, streak_enabled) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (server_id) DO UPDATE SET \
           enabled = $2, xp_per_message = $3, xp_per_reaction = $4, \
           xp_per_voice_min = $5, level_formula = $6, streak_enabled = $7, updated_at = now() \
         RETURNING {GAMIFICATION_CONFIG_COLS}"
    );
    sqlx::query_as::<_, GamificationConfig>(&q)
        .bind(server_id.to_string())
        .bind(enabled)
        .bind(xp_per_message)
        .bind(xp_per_reaction)
        .bind(xp_per_voice_min)
        .bind(level_formula)
        .bind(streak_enabled)
        .fetch_one(pool)
        .await
}

pub async fn get_gamification_config(
    pool: &AnyPool,
    server_id: Uuid,
) -> Result<Option<GamificationConfig>, sqlx::Error> {
    let q = format!(
        "SELECT {GAMIFICATION_CONFIG_COLS} FROM gamification_configs WHERE server_id = $1"
    );
    sqlx::query_as::<_, GamificationConfig>(&q)
        .bind(server_id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn add_xp(
    pool: &AnyPool,
    user_id: Uuid,
    server_id: Uuid,
    amount: i64,
) -> Result<UserXp, sqlx::Error> {
    let q = format!(
        "INSERT INTO user_xp (user_id, server_id, xp) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (user_id, server_id) DO UPDATE SET xp = user_xp.xp + $3, last_xp_at = now() \
         RETURNING {USER_XP_COLS}"
    );
    sqlx::query_as::<_, UserXp>(&q)
        .bind(user_id.to_string())
        .bind(server_id.to_string())
        .bind(amount)
        .fetch_one(pool)
        .await
}

pub async fn get_user_xp(
    pool: &AnyPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<Option<UserXp>, sqlx::Error> {
    let q = format!(
        "SELECT {USER_XP_COLS} FROM user_xp WHERE user_id = $1 AND server_id = $2"
    );
    sqlx::query_as::<_, UserXp>(&q)
        .bind(user_id.to_string())
        .bind(server_id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn list_xp_leaderboard(
    pool: &AnyPool,
    server_id: Uuid,
    limit: i64,
) -> Result<Vec<UserXp>, sqlx::Error> {
    let q = format!(
        "SELECT {USER_XP_COLS} FROM user_xp WHERE server_id = $1 ORDER BY xp DESC LIMIT $2"
    );
    sqlx::query_as::<_, UserXp>(&q)
        .bind(server_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn create_achievement(
    pool: &AnyPool,
    id: Uuid,
    server_id: Uuid,
    name: &str,
    description: Option<&str>,
    icon_url: Option<&str>,
    criteria: &serde_json::Value,
    reward_xp: i32,
) -> Result<Achievement, sqlx::Error> {
    let q = format!(
        "INSERT INTO achievements (id, server_id, name, description, icon_url, criteria, reward_xp) \
         VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7) \
         RETURNING {ACHIEVEMENT_COLS}"
    );
    sqlx::query_as::<_, Achievement>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(name)
        .bind(description)
        .bind(icon_url)
        .bind(criteria.to_string())
        .bind(reward_xp)
        .fetch_one(pool)
        .await
}

pub async fn list_achievements(
    pool: &AnyPool,
    server_id: Uuid,
) -> Result<Vec<Achievement>, sqlx::Error> {
    let q = format!(
        "SELECT {ACHIEVEMENT_COLS} FROM achievements WHERE server_id = $1 ORDER BY name"
    );
    sqlx::query_as::<_, Achievement>(&q)
        .bind(server_id.to_string())
        .fetch_all(pool)
        .await
}

pub async fn award_achievement(
    pool: &AnyPool,
    user_id: Uuid,
    achievement_id: Uuid,
) -> Result<UserAchievement, sqlx::Error> {
    let q = format!(
        "INSERT INTO user_achievements (user_id, achievement_id) \
         VALUES ($1, $2) ON CONFLICT DO NOTHING \
         RETURNING {USER_ACHIEVEMENT_COLS}"
    );
    sqlx::query_as::<_, UserAchievement>(&q)
        .bind(user_id.to_string())
        .bind(achievement_id.to_string())
        .fetch_one(pool)
        .await
}

// ── 23-04: Sync Cursors / Offline Queue ───────────────────────────────────

pub async fn upsert_sync_cursor(
    pool: &AnyPool,
    user_id: Uuid,
    device_id: &str,
    channel_id: Uuid,
    last_message_id: Option<Uuid>,
) -> Result<SyncCursor, sqlx::Error> {
    let q = format!(
        "INSERT INTO sync_cursors (user_id, device_id, channel_id, last_message_id) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (user_id, device_id, channel_id) DO UPDATE SET \
           last_message_id = $4, last_synced_at = now() \
         RETURNING {SYNC_CURSOR_COLS}"
    );
    sqlx::query_as::<_, SyncCursor>(&q)
        .bind(user_id.to_string())
        .bind(device_id)
        .bind(channel_id.to_string())
        .bind(last_message_id.map(|u| u.to_string()))
        .fetch_one(pool)
        .await
}

pub async fn enqueue_offline_action(
    pool: &AnyPool,
    id: Uuid,
    user_id: Uuid,
    device_id: &str,
    action_type: &str,
    payload: &serde_json::Value,
) -> Result<OfflineQueueItem, sqlx::Error> {
    let q = format!(
        "INSERT INTO offline_queue (id, user_id, device_id, action_type, payload) \
         VALUES ($1, $2, $3, $4, $5::jsonb) \
         RETURNING {OFFLINE_QUEUE_ITEM_COLS}"
    );
    sqlx::query_as::<_, OfflineQueueItem>(&q)
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(device_id)
        .bind(action_type)
        .bind(payload.to_string())
        .fetch_one(pool)
        .await
}

pub async fn list_pending_offline(
    pool: &AnyPool,
    user_id: Uuid,
    device_id: &str,
) -> Result<Vec<OfflineQueueItem>, sqlx::Error> {
    let q = format!(
        "SELECT {OFFLINE_QUEUE_ITEM_COLS} FROM offline_queue \
         WHERE user_id = $1 AND device_id = $2 AND status = 'pending' ORDER BY created_at"
    );
    sqlx::query_as::<_, OfflineQueueItem>(&q)
        .bind(user_id.to_string())
        .bind(device_id)
        .fetch_all(pool)
        .await
}
