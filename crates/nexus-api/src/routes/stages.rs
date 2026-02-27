//! Stage channel routes — live speaker + audience events.
//!
//! A stage instance is created on top of an existing `stage`-type channel.
//! A few users speak while the audience can raise their hand to request
//! speak permission.
//!
//! POST   /channels/:id/stage-instance                  — Create stage
//! PATCH  /channels/:id/stage-instance                  — Update topic / privacy
//! DELETE /channels/:id/stage-instance                  — End stage
//! POST   /channels/:id/stage-instance/speakers/:uid    — Invite user to speak (MUTE_MEMBERS)
//! DELETE /channels/:id/stage-instance/speakers/:uid    — Move speaker to audience (MUTE_MEMBERS)
//! POST   /channels/:id/stage-instance/raise-hand       — Audience member requests to speak
//! DELETE /channels/:id/stage-instance/raise-hand       — Retract raise-hand

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::GatewayEvent,
    permissions::Permissions,
    snowflake,
};
use nexus_db::repository::{channels, members, roles, servers};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// ============================================================
// Router
// ============================================================

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Stage instance CRUD
        .route(
            "/channels/{channel_id}/stage-instance",
            post(create_stage)
                .patch(update_stage)
                .delete(end_stage)
                .get(get_stage),
        )
        // Speaker management
        .route(
            "/channels/{channel_id}/stage-instance/speakers/{user_id}",
            post(add_speaker).delete(remove_speaker),
        )
        // Raise hand (audience request to speak)
        .route(
            "/channels/{channel_id}/stage-instance/raise-hand",
            post(raise_hand).delete(lower_hand),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ============================================================
// Models
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageInstance {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub guild_id: Option<Uuid>,
    pub topic: String,
    pub privacy_level: String,
    pub speaker_ids: Vec<Uuid>,
    pub hand_raised_ids: Vec<Uuid>,
    pub started_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateStageRequest {
    topic: Option<String>,
    /// "guild_only" (default) or "public"
    privacy_level: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateStageRequest {
    topic: Option<String>,
    privacy_level: Option<String>,
}

// ============================================================
// DB helper types
// ============================================================

#[derive(sqlx::FromRow)]
struct StageRow {
    id: String,
    channel_id: String,
    guild_id: Option<String>,
    topic: String,
    privacy_level: String,
    speaker_ids: String,     // jsonb stored as text
    hand_raised_ids: String, // jsonb stored as text
    started_at: String,
    created_at: String,
    updated_at: String,
}

impl TryFrom<StageRow> for StageInstance {
    type Error = NexusError;

    fn try_from(r: StageRow) -> Result<Self, Self::Error> {
        let speaker_ids: Vec<Uuid> =
            serde_json::from_str::<Vec<String>>(&r.speaker_ids)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|s| s.parse().ok())
                .collect();

        let hand_raised_ids: Vec<Uuid> =
            serde_json::from_str::<Vec<String>>(&r.hand_raised_ids)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|s| s.parse().ok())
                .collect();

        Ok(StageInstance {
            id: r.id.parse().map_err(|_| NexusError::Internal(anyhow::anyhow!("invalid uuid")))?,
            channel_id: r.channel_id.parse().map_err(|_| NexusError::Internal(anyhow::anyhow!("invalid uuid")))?,
            guild_id: r.guild_id.and_then(|s| s.parse().ok()),
            topic: r.topic,
            privacy_level: r.privacy_level,
            speaker_ids,
            hand_raised_ids,
            started_at: parse_dt(&r.started_at)
                .map_err(|_| NexusError::Internal(anyhow::anyhow!("invalid started_at timestamp")))?,
            created_at: parse_dt(&r.created_at)
                .map_err(|_| NexusError::Internal(anyhow::anyhow!("invalid created_at timestamp")))?,
            updated_at: parse_dt(&r.updated_at)
                .map_err(|_| NexusError::Internal(anyhow::anyhow!("invalid updated_at timestamp")))?,
        })
    }
}

/// Parse a Postgres/SQLite timestamp text representation into `DateTime<Utc>`.
fn parse_dt(s: &str) -> Result<DateTime<Utc>, String> {
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Ok(d.with_timezone(&Utc));
    }
    // Postgres TIMESTAMPTZ::text: "2026-02-22 13:04:47.779907+00"
    let iso = s.replacen(' ', "T", 1);
    let iso = {
        let len = iso.len();
        if len >= 3 {
            let last3 = &iso[len - 3..];
            if (last3.starts_with('+') || last3.starts_with('-'))
                && last3[1..].chars().all(|c| c.is_ascii_digit())
            {
                format!("{}:00", iso)
            } else {
                iso
            }
        } else {
            iso
        }
    };
    if let Ok(d) = DateTime::parse_from_rfc3339(&iso) {
        return Ok(d.with_timezone(&Utc));
    }
    if let Ok(d) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(d.and_utc());
    }
    if let Ok(d) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
        return Ok(d.and_utc());
    }
    Err(format!("cannot parse timestamp '{s}'"))
}

// ============================================================
// Permission helpers
// ============================================================

async fn require_channel_permission(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
    required: Permissions,
) -> NexusResult<Uuid> {
    let channel = channels::find_by_id(&state.db.pool, channel_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

    let server_id = channel.server_id.ok_or(NexusError::MissingPermission {
        permission: format!("{required:?}"),
    })?;

    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Server".into() })?;

    if user_id == server.owner_id {
        return Ok(server_id);
    }

    let member = members::find_member(&state.db.pool, user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or(NexusError::MissingPermission { permission: format!("{required:?}") })?;

    let all_roles = roles::list_server_roles(&state.db.pool, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    let base = all_roles
        .iter()
        .find(|r| r.is_default)
        .map(|r| Permissions::from_bits_truncate(r.permissions))
        .unwrap_or_else(Permissions::empty);

    let effective = all_roles
        .iter()
        .filter(|r| !r.is_default && member.roles.contains(&r.id))
        .map(|r| Permissions::from_bits_truncate(r.permissions))
        .fold(base, |acc, rp| acc | rp);

    if effective.has(required) {
        Ok(server_id)
    } else {
        Err(NexusError::MissingPermission { permission: format!("{required:?}") })
    }
}

/// Fetch the channel's server_id without permission check (for gateway events).
async fn channel_server_id(state: &AppState, channel_id: Uuid) -> Option<Uuid> {
    channels::find_by_id(&state.db.pool, channel_id)
        .await
        .ok()
        .flatten()
        .and_then(|c| c.server_id)
}

// ============================================================
// Stage instance fetch helper
// ============================================================

async fn fetch_stage(pool: &sqlx::AnyPool, channel_id: Uuid) -> NexusResult<StageInstance> {
    let row: Option<StageRow> = sqlx::query_as(
        "SELECT id::text, channel_id::text, guild_id::text, topic, privacy_level, \
              speaker_ids::text, hand_raised_ids::text, \
              started_at::text AS started_at, created_at::text AS created_at, updated_at::text AS updated_at \
         FROM stage_instances WHERE channel_id = $1::uuid",
    )
    .bind(channel_id.to_string())
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or(NexusError::NotFound { resource: "StageInstance".into() })?;
    row.try_into()
}

// ============================================================
// Handlers
// ============================================================

/// GET /api/v1/channels/:channel_id/stage-instance
async fn get_stage(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<StageInstance>> {
    let stage = fetch_stage(&state.db.pool, channel_id).await?;
    Ok(Json(stage))
}

/// POST /api/v1/channels/:channel_id/stage-instance
async fn create_stage(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<CreateStageRequest>,
) -> NexusResult<Json<StageInstance>> {
    // Verify channel is a stage type
    let channel = channels::find_by_id(&state.db.pool, channel_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Channel".into() })?;

    if !matches!(channel.channel_type, nexus_common::models::channel::ChannelType::Stage) {
        return Err(NexusError::Validation {
            message: "Channel is not a stage channel".into(),
        });
    }

    let server_id = require_channel_permission(
        &state,
        channel_id,
        auth.user_id,
        Permissions::MANAGE_CHANNELS,
    )
    .await?;

    // Check no existing stage instance
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id::text FROM stage_instances WHERE channel_id = $1::uuid")
            .bind(channel_id.to_string())
            .fetch_optional(&state.db.pool)
            .await?;

    if existing.is_some() {
        return Err(NexusError::Validation {
            message: "A stage instance already exists for this channel".into(),
        });
    }

    let privacy_level = body
        .privacy_level
        .as_deref()
        .unwrap_or("guild_only")
        .to_string();

    if !matches!(privacy_level.as_str(), "guild_only" | "public") {
        return Err(NexusError::Validation {
            message: "privacy_level must be 'guild_only' or 'public'".into(),
        });
    }

    let topic = body.topic.unwrap_or_default();
    let stage_id = Uuid::new_v4();
    // Initial speaker list contains the creator
    let speaker_ids_json =
        serde_json::to_string(&vec![auth.user_id.to_string()]).unwrap_or_else(|_| "[]".into());

    sqlx::query(
        "INSERT INTO stage_instances \
         (id, channel_id, guild_id, topic, privacy_level, speaker_ids, hand_raised_ids) \
         VALUES ($1::uuid, $2::uuid, $3::uuid, $4, $5, $6, '[]')",
    )
    .bind(stage_id.to_string())
    .bind(channel_id.to_string())
    .bind(server_id.to_string())
    .bind(&topic)
    .bind(&privacy_level)
    .bind(&speaker_ids_json)
    .execute(&state.db.pool)
    .await?;

    let stage = fetch_stage(&state.db.pool, channel_id).await?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: nexus_common::gateway_event::event_types::STAGE_INSTANCE_CREATE.to_string(),
        data: serde_json::to_value(&stage).unwrap_or_default(),
        server_id: Some(server_id),
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(stage))
}

/// PATCH /api/v1/channels/:channel_id/stage-instance
async fn update_stage(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<UpdateStageRequest>,
) -> NexusResult<Json<StageInstance>> {
    let server_id = require_channel_permission(
        &state,
        channel_id,
        auth.user_id,
        Permissions::MANAGE_CHANNELS,
    )
    .await?;

    let current = fetch_stage(&state.db.pool, channel_id).await?;

    let topic = body.topic.as_deref().unwrap_or(&current.topic);
    let privacy_level = body
        .privacy_level
        .as_deref()
        .unwrap_or(&current.privacy_level);

    if !matches!(privacy_level, "guild_only" | "public") {
        return Err(NexusError::Validation {
            message: "privacy_level must be 'guild_only' or 'public'".into(),
        });
    }

    sqlx::query(
        "UPDATE stage_instances SET topic = $1, privacy_level = $2, updated_at = NOW() \
         WHERE channel_id = $3::uuid",
    )
    .bind(topic)
    .bind(privacy_level)
    .bind(channel_id.to_string())
    .execute(&state.db.pool)
    .await?;

    let updated = fetch_stage(&state.db.pool, channel_id).await?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: nexus_common::gateway_event::event_types::STAGE_INSTANCE_UPDATE.to_string(),
        data: serde_json::to_value(&updated).unwrap_or_default(),
        server_id: Some(server_id),
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(updated))
}

/// DELETE /api/v1/channels/:channel_id/stage-instance
async fn end_stage(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let server_id = require_channel_permission(
        &state,
        channel_id,
        auth.user_id,
        Permissions::MANAGE_CHANNELS,
    )
    .await?;

    // Mark the instance as ended (soft delete with ended_at timestamp)
    let rows_affected = sqlx::query(
        "UPDATE stage_instances SET ended_at = NOW(), updated_at = NOW() \
         WHERE channel_id = $1::uuid AND ended_at IS NULL",
    )
    .bind(channel_id.to_string())
    .execute(&state.db.pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(NexusError::NotFound { resource: "StageInstance".into() });
    }

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: nexus_common::gateway_event::event_types::STAGE_INSTANCE_DELETE.to_string(),
        data: serde_json::json!({ "channel_id": channel_id }),
        server_id: Some(server_id),
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    // Physically delete after notifying
    sqlx::query("DELETE FROM stage_instances WHERE channel_id = $1::uuid")
        .bind(channel_id.to_string())
        .execute(&state.db.pool)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// POST /api/v1/channels/:channel_id/stage-instance/speakers/:user_id
/// Invite a user from the audience to the speaker podium (requires MUTE_MEMBERS).
async fn add_speaker(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, target_user_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<StageInstance>> {
    let server_id = require_channel_permission(
        &state, channel_id, auth.user_id, Permissions::MUTE_MEMBERS,
    )
    .await?;

    let stage = fetch_stage(&state.db.pool, channel_id).await?;

    if stage.speaker_ids.contains(&target_user_id) {
        return Err(NexusError::Validation { message: "User is already a speaker".into() });
    }

    let mut new_speakers = stage.speaker_ids.clone();
    new_speakers.push(target_user_id);

    // Also remove from hand_raised list if present
    let new_raised: Vec<Uuid> = stage
        .hand_raised_ids
        .into_iter()
        .filter(|&id| id != target_user_id)
        .collect();

    update_stage_arrays(&state.db.pool, channel_id, &new_speakers, &new_raised).await?;

    let updated = fetch_stage(&state.db.pool, channel_id).await?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: nexus_common::gateway_event::event_types::STAGE_SPEAKER_UPDATE.to_string(),
        data: serde_json::to_value(&updated).unwrap_or_default(),
        server_id: Some(server_id),
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(updated))
}

/// DELETE /api/v1/channels/:channel_id/stage-instance/speakers/:user_id
/// Move a speaker back to the audience (requires MUTE_MEMBERS).
async fn remove_speaker(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((channel_id, target_user_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<StageInstance>> {
    let server_id = require_channel_permission(
        &state, channel_id, auth.user_id, Permissions::MUTE_MEMBERS,
    )
    .await?;

    let stage = fetch_stage(&state.db.pool, channel_id).await?;

    let new_speakers: Vec<Uuid> = stage
        .speaker_ids
        .into_iter()
        .filter(|&id| id != target_user_id)
        .collect();

    update_stage_arrays(&state.db.pool, channel_id, &new_speakers, &stage.hand_raised_ids).await?;

    let updated = fetch_stage(&state.db.pool, channel_id).await?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: nexus_common::gateway_event::event_types::STAGE_SPEAKER_UPDATE.to_string(),
        data: serde_json::to_value(&updated).unwrap_or_default(),
        server_id: Some(server_id),
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(updated))
}

/// POST /api/v1/channels/:channel_id/stage-instance/raise-hand
/// Audience member requests to speak.
async fn raise_hand(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let stage = fetch_stage(&state.db.pool, channel_id).await?;

    if stage.hand_raised_ids.contains(&auth.user_id) {
        return Ok(Json(serde_json::json!({ "raised": true })));
    }

    let mut new_raised = stage.hand_raised_ids.clone();
    new_raised.push(auth.user_id);

    update_stage_arrays(&state.db.pool, channel_id, &stage.speaker_ids, &new_raised).await?;

    let server_id = channel_server_id(&state, channel_id).await;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: nexus_common::gateway_event::event_types::STAGE_SPEAKER_UPDATE.to_string(),
        data: serde_json::json!({
            "channel_id": channel_id,
            "hand_raised_ids": new_raised,
        }),
        server_id,
        channel_id: Some(channel_id),
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "raised": true })))
}

/// DELETE /api/v1/channels/:channel_id/stage-instance/raise-hand
/// Retract a raise-hand request.
async fn lower_hand(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let stage = fetch_stage(&state.db.pool, channel_id).await?;

    let new_raised: Vec<Uuid> = stage
        .hand_raised_ids
        .into_iter()
        .filter(|&id| id != auth.user_id)
        .collect();

    update_stage_arrays(&state.db.pool, channel_id, &stage.speaker_ids, &new_raised).await?;

    Ok(Json(serde_json::json!({ "raised": false })))
}

// ============================================================
// Internal helpers
// ============================================================

/// Overwrite speaker_ids and hand_raised_ids in the DB.
async fn update_stage_arrays(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
    speakers: &[Uuid],
    raised: &[Uuid],
) -> NexusResult<()> {
    let speakers_json = serde_json::to_string(
        &speakers.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".into());

    let raised_json = serde_json::to_string(
        &raised.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".into());

    sqlx::query(
        "UPDATE stage_instances SET speaker_ids = $1, hand_raised_ids = $2, updated_at = NOW() \
         WHERE channel_id = $3::uuid",
    )
    .bind(&speakers_json)
    .bind(&raised_json)
    .bind(channel_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}
