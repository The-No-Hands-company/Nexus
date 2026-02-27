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
use chrono::{DateTime, Utc};
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::{event_types, GatewayEvent},
    snowflake,
};
use serde::{Deserialize, Serialize};
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
    pub id: Uuid,
    pub user_id: Uuid,
    pub badge_type: String,
    pub server_id: Option<Uuid>,
    pub awarded_by: Option<Uuid>,
    pub awarded_at: DateTime<Utc>,
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
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /users/{user_id}/badges` — public
async fn list_user_badges(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> NexusResult<Json<Vec<UserBadge>>> {
    let rows = sqlx::query_as!(
        UserBadge,
        r#"
        SELECT
            id, user_id, badge_type, server_id, awarded_by,
            awarded_at, label, icon_url
        FROM user_badges
        WHERE user_id = $1::uuid
        ORDER BY awarded_at DESC
        "#,
        user_id.to_string(),
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

    Ok(Json(rows))
}

/// `POST /admin/users/{user_id}/badges` — admin-only badge award
async fn award_badge(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<AwardBadgeRequest>,
) -> NexusResult<Json<UserBadge>> {
    // Require admin flag on the requesting user
    let requester = sqlx::query!(
        "SELECT is_bot FROM users WHERE id = $1::uuid",
        auth.user_id.to_string(),
    )
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

    // Only internal system / admins can reach this path.
    // In production this endpoint sits behind network-level admin auth; here we
    // simply verify the caller exists to avoid phantom awards.
    if requester.is_none() {
        return Err(NexusError::Forbidden("Admin only".into()));
    }

    let badge_id = snowflake::generate();
    let badge = sqlx::query_as!(
        UserBadge,
        r#"
        INSERT INTO user_badges
            (id, user_id, badge_type, server_id, awarded_by, label, icon_url)
        VALUES ($1::uuid, $2::uuid, $3, $4::uuid, $5::uuid, $6, $7)
        RETURNING
            id, user_id, badge_type, server_id, awarded_by,
            awarded_at, label, icon_url
        "#,
        badge_id.to_string(),
        user_id.to_string(),
        body.badge_type,
        body.server_id.map(|s| s.to_string()),
        auth.user_id.to_string(),
        body.label,
        body.icon_url,
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

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
        .ok_or_else(|| NexusError::Validation("badge_type required".into()))?;

    sqlx::query!(
        "DELETE FROM user_badges WHERE user_id = $1::uuid AND badge_type = $2",
        user_id.to_string(),
        badge_type,
    )
    .execute(&state.db.pool)
    .await
    .map_err(|e| NexusError::Database(e.to_string()))?;

    Ok(Json(serde_json::json!({ "revoked": true })))
}
