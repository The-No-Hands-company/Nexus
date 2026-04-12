//! User growth & retention routes — Phase 23.
//!
//! Server recommendations, onboarding flows, device sessions,
//! gamification, achievements, sync cursors, offline queue.

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::models::growth::*;
use nexus_db::repository::{growth, members};
use nexus_common::models::Member;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// ── Router ─────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Server recommendations
        .route(
            "/users/@me/recommendations",
            get(list_recommendations),
        )
        .route(
            "/users/@me/recommendations/{rec_id}/dismiss",
            post(dismiss_recommendation),
        )
        // Onboarding flows
        .route(
            "/servers/{server_id}/onboarding-flow",
            get(get_onboarding_flow).post(upsert_onboarding_flow),
        )
        // Device sessions
        .route(
            "/users/@me/device-sessions",
            get(list_device_sessions).post(upsert_device_session),
        )
        // Gamification config
        .route(
            "/servers/{server_id}/gamification",
            get(get_gamification_config).post(upsert_gamification_config),
        )
        // XP + leaderboard
        .route("/servers/{server_id}/xp", get(get_user_xp_handler).post(add_xp_handler))
        .route("/servers/{server_id}/xp/leaderboard", get(list_xp_leaderboard))
        // Achievements
        .route(
            "/servers/{server_id}/achievements",
            get(list_achievements).post(create_achievement),
        )
        .route(
            "/servers/{server_id}/achievements/{achievement_id}/award",
            post(award_achievement),
        )
        // Sync cursors
        .route("/users/@me/sync-cursors", get(get_sync_cursor).post(upsert_sync_cursor))
        // Offline queue
        .route("/users/@me/offline-queue", get(list_pending_offline).post(enqueue_offline))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request / Query ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LimitQuery { limit: Option<i64>, device_id: Option<String> }

#[derive(Debug, Deserialize)]
struct UpsertOnboardingFlowReq {
    steps: serde_json::Value,
    adaptive: Option<bool>,
    skip_completed: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpsertDeviceSessionReq {
    device_id: String,
    device_type: String,
    last_channel_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct UpsertGamificationConfigReq {
    enabled: Option<bool>,
    xp_per_message: Option<i32>,
    xp_per_reaction: Option<i32>,
    xp_per_voice_min: Option<i32>,
    level_formula: Option<String>,
    streak_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AddXpReq {
    amount: i64,
}

#[derive(Debug, Deserialize)]
struct CreateAchievementReq {
    name: String,
    description: Option<String>,
    icon_url: Option<String>,
    criteria: Option<serde_json::Value>,
    xp_reward: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct UpsertSyncCursorReq {
    device_id: String,
    channel_id: Uuid,
    last_message_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct EnqueueOfflineReq {
    device_id: String,
    action_type: String,
    payload: serde_json::Value,
}

// ── Handlers ──────────────────────────────────────────────────────────────

async fn list_recommendations(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<ServerRecommendation>>> {
    let rows = growth::list_recommendations(&state.db.pool, ctx.user_id, q.limit.unwrap_or(20))
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn dismiss_recommendation(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(rec_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    growth::dismiss_recommendation(&state.db.pool, ctx.user_id, rec_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn get_onboarding_flow(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Option<OnboardingFlow>>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let row = growth::get_onboarding_flow(&state.db.pool, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn upsert_onboarding_flow(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<UpsertOnboardingFlowReq>,
) -> NexusResult<Json<OnboardingFlow>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let row = growth::upsert_onboarding_flow(
        &state.db.pool, Uuid::new_v4(), server_id, &body.steps,
        body.adaptive.unwrap_or(false),
        body.skip_completed.unwrap_or(true),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_device_sessions(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<DeviceSession>>> {
    let rows = growth::list_device_sessions(&state.db.pool, ctx.user_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn upsert_device_session(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<UpsertDeviceSessionReq>,
) -> NexusResult<Json<DeviceSession>> {
    let row = growth::upsert_device_session(
        &state.db.pool, Uuid::new_v4(), ctx.user_id,
        &body.device_id, &body.device_type,
        body.last_channel_id,
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn get_gamification_config(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Option<GamificationConfig>>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let row = growth::get_gamification_config(&state.db.pool, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn upsert_gamification_config(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<UpsertGamificationConfigReq>,
) -> NexusResult<Json<GamificationConfig>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let row = growth::upsert_gamification_config(
        &state.db.pool, server_id,
        body.enabled.unwrap_or(true),
        body.xp_per_message.unwrap_or(1),
        body.xp_per_reaction.unwrap_or(1),
        body.xp_per_voice_min.unwrap_or(2),
        &body.level_formula.unwrap_or_else(|| "linear".into()),
        body.streak_enabled.unwrap_or(false),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn get_user_xp_handler(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Option<UserXp>>> {
    let row = growth::get_user_xp(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn add_xp_handler(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<AddXpReq>,
) -> NexusResult<Json<UserXp>> {
    let row = growth::add_xp(
        &state.db.pool, ctx.user_id, server_id, body.amount,
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_xp_leaderboard(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<UserXp>>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let rows = growth::list_xp_leaderboard(&state.db.pool, server_id, q.limit.unwrap_or(50))
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn list_achievements(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<Achievement>>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let rows = growth::list_achievements(&state.db.pool, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_achievement(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateAchievementReq>,
) -> NexusResult<Json<Achievement>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let row = growth::create_achievement(
        &state.db.pool, Uuid::new_v4(), server_id, &body.name,
        body.description.as_deref(), body.icon_url.as_deref(),
        &body.criteria.unwrap_or(serde_json::json!({})),
        body.xp_reward.unwrap_or(0),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn award_achievement(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path((server_id, achievement_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<UserAchievement>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let row = growth::award_achievement(&state.db.pool, ctx.user_id, achievement_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn get_sync_cursor(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Query(q): Query<SyncCursorQuery>,
) -> NexusResult<Json<Option<SyncCursor>>> {
    let row = growth::get_sync_cursor(
        &state.db.pool, ctx.user_id, &q.device_id, q.channel_id,
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

#[derive(Debug, Deserialize)]
struct SyncCursorQuery { device_id: String, channel_id: Uuid }

async fn upsert_sync_cursor(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<UpsertSyncCursorReq>,
) -> NexusResult<Json<SyncCursor>> {
    let row = growth::upsert_sync_cursor(
        &state.db.pool, ctx.user_id, &body.device_id, body.channel_id, body.last_message_id,
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_pending_offline(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<OfflineQueueItem>>> {
    let device_id = q.device_id.as_deref().unwrap_or("default");
    let rows = growth::list_pending_offline(&state.db.pool, ctx.user_id, device_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn enqueue_offline(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<EnqueueOfflineReq>,
) -> NexusResult<Json<OfflineQueueItem>> {
    let row = growth::enqueue_offline_action(
        &state.db.pool, Uuid::new_v4(), ctx.user_id, &body.device_id, &body.action_type, &body.payload,
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}
