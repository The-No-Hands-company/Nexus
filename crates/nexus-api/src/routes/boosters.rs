//! Server supporter / boost tier routes — v0.15 Community Ecosystem.
//!
//! POST   /servers/{id}/boost               — add a boost slot (authenticated user)
//! DELETE /servers/{id}/boost/{slot}        — remove a boost slot (authenticated user)
//! GET    /servers/{id}/boosters            — list active boosters
//! GET    /servers/{id}/boost-tier          — get current tier + perks summary
//! PATCH  /servers/{id}/vanity-url          — set vanity invite code (MANAGE_SERVER, tier 2+)

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::{event_types, GatewayEvent},
    permissions::Permissions,
    snowflake,
};
use nexus_db::repository::{members, roles};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// Boost count thresholds for each tier
const TIER1_THRESHOLD: i64 = 2;
const TIER2_THRESHOLD: i64 = 7;
const TIER3_THRESHOLD: i64 = 14;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/servers/{server_id}/boost", post(add_boost))
        .route("/servers/{server_id}/boost/{slot}", delete(remove_boost))
        .route("/servers/{server_id}/boosters", get(list_boosters))
        .route("/servers/{server_id}/boost-tier", get(get_boost_tier))
        .route("/servers/{server_id}/vanity-url", patch(set_vanity_url))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ─────────────────────────────────────────────────────────────────────────────
// Models
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct BoosterEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub server_id: Uuid,
    pub slot: i16,
    pub started_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct BoostTierInfo {
    pub tier: i16,
    pub booster_count: i64,
    /// Extra emoji slots granted: tier1=+50, tier2=+100, tier3=+200
    pub extra_emoji_slots: i32,
    /// Upload limit in bytes (tier0=8MB, tier1=25MB, tier2=50MB, tier3=100MB)
    pub upload_limit_bytes: i64,
    /// Whether vanity URL is available (tier 2+)
    pub vanity_url_available: bool,
    pub current_vanity_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetVanityUrlRequest {
    /// Desired vanity code (2-32 alphanumeric + hyphen)
    pub code: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tier helpers
// ─────────────────────────────────────────────────────────────────────────────

fn tier_from_count(count: i64) -> i16 {
    if count >= TIER3_THRESHOLD { 3 }
    else if count >= TIER2_THRESHOLD { 2 }
    else if count >= TIER1_THRESHOLD { 1 }
    else { 0 }
}

fn perks(tier: i16, vanity_code: Option<String>) -> BoostTierInfo {
    let (extra_emoji, upload_limit) = match tier {
        1 => (50, 25 * 1024 * 1024),
        2 => (100, 50 * 1024 * 1024),
        3 => (200, 100 * 1024 * 1024),
        _ => (0, 8 * 1024 * 1024),
    };
    BoostTierInfo {
        tier,
        booster_count: 0, // filled in caller
        extra_emoji_slots: extra_emoji,
        upload_limit_bytes: upload_limit,
        vanity_url_available: tier >= 2,
        current_vanity_code: vanity_code,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /servers/{server_id}/boost` — add a boost (slot auto-assigned 1 or 2)
async fn add_boost(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<BoosterEntry>> {
    // Find a free slot (1 or 2) for this user on this server
    let used_slots: Vec<i16> = sqlx::query_scalar!(
        r#"SELECT slot FROM server_boosters WHERE user_id = $1::uuid AND server_id = $2::uuid AND (expires_at IS NULL OR expires_at > NOW())"#,
        auth.user_id.to_string(),
        server_id.to_string(),
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

    let slot = if !used_slots.contains(&1) { 1i16 }
    else if !used_slots.contains(&2) { 2i16 }
    else {
        return Err(NexusError::Validation("Both boost slots already used on this server".into()));
    };

    let boost_id = snowflake::generate();
    let entry = sqlx::query_as!(
        BoosterEntry,
        r#"
        INSERT INTO server_boosters (id, user_id, server_id, slot)
        VALUES ($1::uuid, $2::uuid, $3::uuid, $4)
        RETURNING id, user_id, server_id, slot, started_at, expires_at
        "#,
        boost_id.to_string(),
        auth.user_id.to_string(),
        server_id.to_string(),
        slot,
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

    // Recompute and persist tier
    recalculate_tier(&state, server_id).await?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::SERVER_BOOST.to_string(),
        data: serde_json::to_value(&entry).unwrap_or_default(),
        server_id: Some(server_id),
        channel_id: None,
        user_id: Some(auth.user_id),
    });

    Ok(Json(entry))
}

/// `DELETE /servers/{server_id}/boost/{slot}` — remove a specific boost slot
async fn remove_boost(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path((server_id, slot)): Path<(Uuid, i16)>,
) -> NexusResult<Json<serde_json::Value>> {
    let deleted = sqlx::query!(
        r#"
        DELETE FROM server_boosters
        WHERE user_id = $1::uuid AND server_id = $2::uuid AND slot = $3
        "#,
        auth.user_id.to_string(),
        server_id.to_string(),
        slot,
    )
    .execute(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

    if deleted.rows_affected() == 0 {
        return Err(NexusError::NotFound("Boost slot not found".into()));
    }

    recalculate_tier(&state, server_id).await?;

    Ok(Json(serde_json::json!({ "removed": true })))
}

/// `GET /servers/{server_id}/boosters` — list active boosters
async fn list_boosters(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<BoosterEntry>>> {
    let _ = auth;
    let boosters = sqlx::query_as!(
        BoosterEntry,
        r#"
        SELECT id, user_id, server_id, slot, started_at, expires_at
        FROM server_boosters
        WHERE server_id = $1::uuid
          AND (expires_at IS NULL OR expires_at > NOW())
        ORDER BY started_at ASC
        "#,
        server_id.to_string(),
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

    Ok(Json(boosters))
}

/// `GET /servers/{server_id}/boost-tier` — current tier + perks
async fn get_boost_tier(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<BoostTierInfo>> {
    let _ = auth;
    let row = sqlx::query!(
        "SELECT boost_tier, booster_count, vanity_code FROM servers WHERE id = $1::uuid",
        server_id.to_string(),
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?
    .ok_or_else(|| NexusError::NotFound("Server not found".into()))?;

    let mut info = perks(row.boost_tier as i16, row.vanity_code);
    info.booster_count = row.booster_count as i64;
    Ok(Json(info))
}

/// `PATCH /servers/{server_id}/vanity-url` — set vanity invite code (tier 2+, MANAGE_SERVER)
async fn set_vanity_url(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<SetVanityUrlRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    // Check tier
    let row = sqlx::query!(
        "SELECT boost_tier, owner_id FROM servers WHERE id = $1::uuid",
        server_id.to_string(),
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?
    .ok_or_else(|| NexusError::NotFound("Server not found".into()))?;

    if row.boost_tier < 2 {
        return Err(NexusError::Forbidden("Tier 2 or higher required for vanity URL".into()));
    }

    // MANAGE_SERVER permission check
    let is_owner = row.owner_id.parse::<Uuid>().ok() == Some(auth.user_id);
    if !is_owner {
        let member_roles = members::get_member_roles(&state.db.pool, server_id, auth.user_id)
            .await
            .map_err(|e| NexusError::Database(e.to_string()))?;
        let perms = roles::calculate_permissions(&state.db.pool, server_id, &member_roles)
            .await
            .map_err(|e| NexusError::Database(e.to_string()))?;
        let bits = Permissions::from_bits_truncate(perms);
        if !bits.contains(Permissions::MANAGE_SERVER) && !bits.contains(Permissions::ADMINISTRATOR) {
            return Err(NexusError::Forbidden("MANAGE_SERVER required".into()));
        }
    }

    // Validate code format (2-32 chars, alphanumeric + hyphen)
    let code = body.code.trim();
    if code.len() < 2 || code.len() > 32 || !code.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(NexusError::Validation("Vanity code must be 2-32 alphanumeric characters or hyphens".into()));
    }

    sqlx::query!(
        "UPDATE servers SET vanity_code = $1 WHERE id = $2::uuid",
        code,
        server_id.to_string(),
    )
    .execute(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({ "vanity_code": code })))
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Recount active boosts, compute new tier, persist to servers table, and emit
/// SERVER_TIER_UPDATE if the tier changed.
async fn recalculate_tier(state: &AppState, server_id: Uuid) -> NexusResult<()> {
    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM server_boosters WHERE server_id = $1::uuid AND (expires_at IS NULL OR expires_at > NOW())",
        server_id.to_string(),
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?
    .unwrap_or(0);

    let new_tier = tier_from_count(count) as i32;

    let changed = sqlx::query_scalar!(
        r#"
        UPDATE servers
        SET boost_tier    = $1,
            booster_count = $2
        WHERE id = $3::uuid
          AND (boost_tier != $1 OR booster_count != $2)
        RETURNING boost_tier
        "#,
        new_tier,
        count as i32,
        server_id.to_string(),
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

    // Only broadcast if tier actually changed
    if changed.is_some() {
        let _ = state.gateway_tx.send(GatewayEvent {
            event_type: event_types::SERVER_TIER_UPDATE.to_string(),
            data: serde_json::json!({
                "server_id": server_id,
                "tier": new_tier,
                "booster_count": count,
            }),
            server_id: Some(server_id),
            channel_id: None,
            user_id: None,
        });
    }

    Ok(())
}
