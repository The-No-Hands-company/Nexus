//! On-device AI assist routes — 16-04.
//!
//! The actual AI inference runs on-device (privacy-first). The server only
//! stores per-user preferences and channel digest history.
//!
//! GET    /users/@me/ai-preferences                   — Get preferences
//! PUT    /users/@me/ai-preferences                   — Update preferences
//! GET    /channels/:id/digests                        — List channel digests
//! POST   /channels/:id/digests                        — Save a new digest

use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    middleware,
    routing::get,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::models::collaboration::{AiPreferences, ChannelDigest};
use nexus_common::snowflake;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{AppState, middleware::AuthContext};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/users/@me/ai-preferences",
            get(get_preferences).put(update_preferences),
        )
        .route(
            "/channels/{channel_id}/digests",
            get(list_digests).post(save_digest),
        )
        .route_layer(middleware::from_fn(
            crate::middleware::combined_auth_middleware,
        ))
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct UpdatePrefsRequest {
    summaries_enabled: Option<bool>,
    smart_replies: Option<bool>,
    auto_mod_suggest: Option<bool>,
    digest_enabled: Option<bool>,
    digest_interval: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SaveDigestRequest {
    period_start: String,
    period_end: String,
    summary: String,
    message_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct DigestsQuery {
    limit: Option<i64>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn get_preferences(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<AiPreferences>> {
    let row = sqlx::query_as::<_, AiPreferences>(
        "INSERT INTO ai_preferences (user_id) VALUES ($1) \
         ON CONFLICT (user_id) DO NOTHING; \
         SELECT user_id::text AS user_id, summaries_enabled, smart_replies, \
                auto_mod_suggest, digest_enabled, digest_interval, \
                updated_at::text AS updated_at \
         FROM ai_preferences WHERE user_id = $1",
    )
    .bind(ctx.user_id.to_string())
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(row))
}

async fn update_preferences(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<UpdatePrefsRequest>,
) -> NexusResult<Json<AiPreferences>> {
    if let Some(ref interval) = body.digest_interval
        && !matches!(interval.as_str(), "daily" | "weekly") {
            return Err(NexusError::Validation {
                message: "digest_interval must be 'daily' or 'weekly'".into(),
            });
        }

    let row = sqlx::query_as::<_, AiPreferences>(
        "INSERT INTO ai_preferences (user_id) VALUES ($1) \
         ON CONFLICT (user_id) DO NOTHING; \
         UPDATE ai_preferences SET \
         summaries_enabled = COALESCE($2, summaries_enabled), \
         smart_replies = COALESCE($3, smart_replies), \
         auto_mod_suggest = COALESCE($4, auto_mod_suggest), \
         digest_enabled = COALESCE($5, digest_enabled), \
         digest_interval = COALESCE($6, digest_interval), \
         updated_at = NOW() \
         WHERE user_id = $1 \
         RETURNING user_id::text AS user_id, summaries_enabled, smart_replies, \
                   auto_mod_suggest, digest_enabled, digest_interval, \
                   updated_at::text AS updated_at",
    )
    .bind(ctx.user_id.to_string())
    .bind(body.summaries_enabled)
    .bind(body.smart_replies)
    .bind(body.auto_mod_suggest)
    .bind(body.digest_enabled)
    .bind(body.digest_interval.as_deref())
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(row))
}

async fn list_digests(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Query(q): Query<DigestsQuery>,
) -> NexusResult<Json<Vec<ChannelDigest>>> {
    let limit = q.limit.unwrap_or(20).min(100);

    let rows = sqlx::query_as::<_, ChannelDigest>(
        "SELECT id::text AS id, channel_id::text AS channel_id, user_id::text AS user_id, \
                period_start::text AS period_start, period_end::text AS period_end, \
                summary, message_count, created_at::text AS created_at \
         FROM channel_digests \
         WHERE channel_id = $1 AND user_id = $2 \
         ORDER BY period_end DESC LIMIT $3",
    )
    .bind(channel_id.to_string())
    .bind(ctx.user_id.to_string())
    .bind(limit)
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(rows))
}

async fn save_digest(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<SaveDigestRequest>,
) -> NexusResult<Json<ChannelDigest>> {
    if body.summary.is_empty() || body.summary.len() > 10_000 {
        return Err(NexusError::Validation {
            message: "Summary must be 1-10000 characters".into(),
        });
    }

    let row = sqlx::query_as::<_, ChannelDigest>(
        "INSERT INTO channel_digests \
         (id, channel_id, user_id, period_start, period_end, summary, message_count) \
         VALUES ($1, $2, $3, $4::timestamptz, $5::timestamptz, $6, $7) \
         RETURNING id::text AS id, channel_id::text AS channel_id, user_id::text AS user_id, \
                   period_start::text AS period_start, period_end::text AS period_end, \
                   summary, message_count, created_at::text AS created_at",
    )
    .bind(snowflake::generate_id().to_string())
    .bind(channel_id.to_string())
    .bind(ctx.user_id.to_string())
    .bind(&body.period_start)
    .bind(&body.period_end)
    .bind(&body.summary)
    .bind(body.message_count.unwrap_or(0))
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(row))
}
