//! Enhanced presence routes — rich status, activity, custom emoji.
//!
//! POST /users/@me/presence — Update presence, custom status, and activity
//! GET  /users/:id/presence — Get a user's public presence

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::rich::UpdatePresenceRequest,
    validation::validate_request,
};
use nexus_db::repository::users;
use nexus_common::gateway_event::GatewayEvent;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// Module-level helper structs for sqlx queries (cannot be defined inside async fns)
#[derive(sqlx::FromRow)]
struct UserCustomEmojiRow { custom_status_emoji: Option<String> }

#[derive(sqlx::FromRow)]
struct UserActivityRow {
    activity_type: Option<String>,
    name: Option<String>,
    details: Option<String>,
    state: Option<String>,
    url: Option<String>,
    large_image: Option<String>,
    small_image: Option<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users/@me/presence", post(update_presence))
        .route("/users/{user_id}/presence", get(get_user_presence))
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

// ============================================================
// Response
// ============================================================

#[derive(Serialize)]
struct PresenceResponse {
    user_id: Uuid,
    presence: nexus_common::models::user::UserPresence,
    status: Option<String>,
    custom_status_emoji: Option<String>,
    activity: Option<ActivityResponse>,
}

#[derive(Serialize)]
struct ActivityResponse {
    activity_type: Option<String>,
    name: Option<String>,
    details: Option<String>,
    state: Option<String>,
    url: Option<String>,
    large_image: Option<String>,
    small_image: Option<String>,
}

// ============================================================
// POST /users/@me/presence
// ============================================================

async fn update_presence(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdatePresenceRequest>,
) -> NexusResult<Json<PresenceResponse>> {
    validate_request(&body)?;

    // Update presence + status in the users table
    if body.presence.is_some() || body.status.is_some() || body.custom_status_emoji.is_some() {
        // Use string-based presence cast to avoid compile-time type checking
        let presence_str = body.presence.map(|p| format!("{:?}", p).to_lowercase());
        let presence_str = presence_str.as_deref();
        sqlx::query(
            r#"
            UPDATE users
            SET
                presence = COALESCE(CAST($2 AS user_presence), presence),
                status = COALESCE($3, status),
                custom_status_emoji = COALESCE($4, custom_status_emoji),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(auth.user_id)
        .bind(presence_str)
        .bind(body.status.as_deref())
        .bind(body.custom_status_emoji.as_deref())
        .execute(&state.db.pg)
        .await?;
    }

    // Upsert the activity row
    let activity_resp = if let Some(ref act) = body.activity {
        sqlx::query(
            r#"
            INSERT INTO user_activities (
                user_id, activity_type, name, details,
                state, url, large_image, small_image,
                started_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
            ON CONFLICT (user_id) DO UPDATE SET
                activity_type = EXCLUDED.activity_type,
                name = EXCLUDED.name,
                details = EXCLUDED.details,
                state = EXCLUDED.state,
                url = EXCLUDED.url,
                large_image = EXCLUDED.large_image,
                small_image = EXCLUDED.small_image,
                updated_at = NOW()
            "#,
        )
        .bind(auth.user_id)
        .bind(act.activity_type.as_deref())
        .bind(act.name.as_deref())
        .bind(act.details.as_deref())
        .bind(act.state.as_deref())
        .bind(act.url.as_deref())
        .bind(act.large_image.as_deref())
        .bind(act.small_image.as_deref())
        .execute(&state.db.pg)
        .await?;

        Some(ActivityResponse {
            activity_type: act.activity_type.clone(),
            name: act.name.clone(),
            details: act.details.clone(),
            state: act.state.clone(),
            url: act.url.clone(),
            large_image: act.large_image.clone(),
            small_image: act.small_image.clone(),
        })
    } else {
        None
    };

    // Fetch the updated user
    let user = users::find_by_id(&state.db.pg, auth.user_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "User".into(),
        })?;

    let custom_emoji = sqlx::query_as::<_, UserCustomEmojiRow>(
        "SELECT custom_status_emoji FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db.pg)
    .await?
    .and_then(|r| r.custom_status_emoji);

    // Broadcast presence update to gateway
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: "PRESENCE_UPDATE".into(),
        data: serde_json::json!({
            "user_id": auth.user_id,
            "presence": user.presence,
            "status": user.status,
            "custom_status_emoji": custom_emoji,
            "activity": activity_resp.as_ref().map(|a| serde_json::json!({
                "type": a.activity_type,
                "name": a.name,
                "details": a.details,
                "state": a.state,
            }))
        }),
        server_id: None,
        channel_id: None,
        user_id: Some(auth.user_id),
    });

    Ok(Json(PresenceResponse {
        user_id: auth.user_id,
        presence: user.presence,
        status: user.status,
        custom_status_emoji: custom_emoji,
        activity: activity_resp,
    }))
}

// ============================================================
// GET /users/:user_id/presence
// ============================================================

async fn get_user_presence(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> NexusResult<Json<PresenceResponse>> {
    let _ = auth;

    let user = users::find_by_id(&state.db.pg, user_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "User".into(),
        })?;

    let custom_emoji = sqlx::query_as::<_, UserCustomEmojiRow>(
        "SELECT custom_status_emoji FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.db.pg)
    .await?
    .and_then(|r| r.custom_status_emoji);

    let activity_resp = sqlx::query_as::<_, UserActivityRow>(
        r#"
        SELECT activity_type, name, details, state, url, large_image, small_image
        FROM user_activities
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db.pg)
    .await?
    .map(|r| ActivityResponse {
        activity_type: r.activity_type,
        name: r.name,
        details: r.details,
        state: r.state,
        url: r.url,
        large_image: r.large_image,
        small_image: r.small_image,
    });

    Ok(Json(PresenceResponse {
        user_id: user.id,
        presence: user.presence,
        status: user.status,
        custom_status_emoji: custom_emoji,
        activity: activity_resp,
    }))
}
