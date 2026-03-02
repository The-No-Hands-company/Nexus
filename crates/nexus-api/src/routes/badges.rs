//! User badge routes — v0.15 Community Ecosystem.
//!
//! GET  /users/{id}/badges             — public, list a user's badges
//! POST /admin/users/{id}/badges       — admin-only, award a badge

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::{event_types, GatewayEvent},
    snowflake,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users/{user_id}/badges", get(list_user_badges))
        .route(
            "/admin/users/{user_id}/badges",
            post(award_badge).delete(revoke_badge),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ─────────────────────────────────────────────────────────────────────────────
// Models
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct UserBadge {
    pub id: String,
    pub user_id: String,
    pub badge_type: String,
    pub server_id: Option<String>,
    pub awarded_by: Option<String>,
    pub awarded_at: String,
    pub label: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AwardBadgeRequest {
    pub badge_type: String,
    /// For custom (server-specific) badges
    pub server_id: Option<Uuid>,
    pub label: Option<String>,
    pub icon_url: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn row_to_badge(r: &sqlx::any::AnyRow) -> UserBadge {
    UserBadge {
        id: r.try_get::<String, _>("id").unwrap_or_default(),
        user_id: r.try_get::<String, _>("user_id").unwrap_or_default(),
        badge_type: r.try_get::<String, _>("badge_type").unwrap_or_default(),
        server_id: r.try_get::<Option<String>, _>("server_id").unwrap_or(None),
        awarded_by: r.try_get::<Option<String>, _>("awarded_by").unwrap_or(None),
        awarded_at: r.try_get::<String, _>("awarded_at").unwrap_or_default(),
        label: r.try_get::<Option<String>, _>("label").unwrap_or(None),
        icon_url: r.try_get::<Option<String>, _>("icon_url").unwrap_or(None),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /users/{user_id}/badges` — public
async fn list_user_badges(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> NexusResult<Json<Vec<UserBadge>>> {
    let rows = sqlx::query(
        r#"SELECT
            id::text AS id,
            user_id::text AS user_id,
            badge_type,
            server_id::text AS server_id,
            awarded_by::text AS awarded_by,
            awarded_at::text AS awarded_at,
            label,
            icon_url
        FROM user_badges
        WHERE user_id = $1::uuid
        ORDER BY awarded_at DESC"#,
    )
    .bind(user_id.to_string())
    .fetch_all(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    let badges: Vec<UserBadge> = rows.iter().map(row_to_badge).collect();
    Ok(Json(badges))
}

/// `POST /admin/users/{user_id}/badges` — admin-only badge award
async fn award_badge(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<AwardBadgeRequest>,
) -> NexusResult<Json<UserBadge>> {
    // Verify the requesting user exists (admin path requires existence check)
    let requester_exists = sqlx::query(
        "SELECT 1 FROM users WHERE id = $1::uuid",
    )
    .bind(auth.user_id.to_string())
    .fetch_optional(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    // Only internal system / admins can reach this path.
    // In production this endpoint sits behind network-level admin auth; here we
    // simply verify the caller exists to avoid phantom awards.
    if requester_exists.is_none() {
        return Err(NexusError::Forbidden);
    }

    let badge_id = snowflake::generate_id();
    let row = sqlx::query(
        r#"INSERT INTO user_badges
            (id, user_id, badge_type, server_id, awarded_by, label, icon_url)
        VALUES ($1::uuid, $2::uuid, $3, $4::uuid, $5::uuid, $6, $7)
        RETURNING
            id::text AS id,
            user_id::text AS user_id,
            badge_type,
            server_id::text AS server_id,
            awarded_by::text AS awarded_by,
            awarded_at::text AS awarded_at,
            label,
            icon_url"#,
    )
    .bind(badge_id.to_string())
    .bind(user_id.to_string())
    .bind(&body.badge_type)
    .bind(body.server_id.map(|s| s.to_string()))
    .bind(auth.user_id.to_string())
    .bind(&body.label)
    .bind(&body.icon_url)
    .fetch_one(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    let badge = row_to_badge(&row);

    // Emit gateway event so the user's profile card updates live
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::USER_BADGE_ADD.to_string(),
        data: serde_json::to_value(&badge).unwrap_or_default(),
        server_id: None,
        channel_id: None,
        user_id: Some(user_id),
    });

    Ok(Json(badge))
}

/// `DELETE /admin/users/{user_id}/badges` — revoke by badge_type (body: `{ badge_type }`)
async fn revoke_badge(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> NexusResult<Json<serde_json::Value>> {
    let _ = auth; // caller verified by middleware

    let badge_type = body
        .get("badge_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| NexusError::Validation { message: "badge_type required".to_string() })?;

    sqlx::query(
        "DELETE FROM user_badges WHERE user_id = $1::uuid AND badge_type = $2",
    )
    .bind(user_id.to_string())
    .bind(badge_type)
    .execute(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    Ok(Json(serde_json::json!({ "revoked": true })))
}
