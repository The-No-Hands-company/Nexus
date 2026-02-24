//! User routes — profile management, user lookup.

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::get,
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::user::{UpdateUserRequest, UserResponse},
    validation::validate_request,
};
use nexus_db::repository::users;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

/// User routes (all require authentication).
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users/@me", get(get_current_user).patch(update_current_user))
        .route("/users/{user_id}", get(get_user))
        .route("/users/{user_id}/profile", get(get_user_profile))
        .route_layer(middleware::from_fn(
            crate::middleware::auth_middleware,
        ))
}

/// GET /api/v1/users/@me — Get the authenticated user's profile.
async fn get_current_user(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<UserResponse>> {
    let user = users::find_by_id(&state.db.pool, auth.user_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "User".into(),
        })?;

    Ok(Json(user.into()))
}

/// PATCH /api/v1/users/@me — Update the authenticated user's profile.
async fn update_current_user(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateUserRequest>,
) -> NexusResult<Json<UserResponse>> {
    validate_request(&body)?;

    // If changing username, check availability
    if let Some(ref new_username) = body.username {
        if let Some(existing) = users::find_by_username(&state.db.pool, new_username).await? {
            if existing.id != auth.user_id {
                return Err(NexusError::AlreadyExists {
                    resource: "Username".into(),
                });
            }
        }
    }

    let user = users::update_user(
        &state.db.pool,
        auth.user_id,
        body.username.as_deref(),
        body.display_name.as_deref(),
        body.bio.as_deref(),
        body.status.as_deref(),
    )
    .await?;

    Ok(Json(user.into()))
}

/// GET /api/v1/users/:user_id — Get a user's public profile.
async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> NexusResult<Json<UserResponse>> {
    let user = users::find_by_id(&state.db.pool, user_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "User".into(),
        })?;

    Ok(Json(user.into()))
}

/// GET /api/v1/users/:user_id/profile — Enriched profile with mutual servers/friends.
async fn get_user_profile(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let user = users::find_by_id(&state.db.pool, user_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "User".into() })?;

    // Servers in common — both requesting user and target user are members
    let common_rows = sqlx::query(
        "SELECT s.id::text AS id, s.name, s.icon, s.member_count \
         FROM servers s \
         JOIN members m1 ON m1.server_id = s.id AND m1.user_id = $1::uuid \
         JOIN members m2 ON m2.server_id = s.id AND m2.user_id = $2::uuid \
         ORDER BY s.name LIMIT 20",
    )
    .bind(auth.user_id.to_string())
    .bind(user_id.to_string())
    .fetch_all(&state.db.pool)
    .await?;

    use sqlx::Row;
    let servers_in_common: Vec<serde_json::Value> = common_rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.try_get::<String, _>("id").unwrap_or_default(),
                "name": row.try_get::<String, _>("name").unwrap_or_default(),
                "icon": row.try_get::<Option<String>, _>("icon").unwrap_or(None),
                "member_count": row.try_get::<i64, _>("member_count").unwrap_or_default(),
            })
        })
        .collect();

    // Mutual friends — accepted friends of both the requesting user and the target user
    let mutual_rows = sqlx::query(
        "SELECT DISTINCT u.id::text AS id, u.username, u.display_name, u.avatar \
         FROM users u \
         WHERE u.id != $1::uuid AND u.id != $2::uuid \
         AND EXISTS ( \
             SELECT 1 FROM user_relationships \
             WHERE ((requester_id = $1::uuid AND addressee_id = u.id) \
                    OR (addressee_id = $1::uuid AND requester_id = u.id)) \
             AND status = 'accepted') \
         AND EXISTS ( \
             SELECT 1 FROM user_relationships \
             WHERE ((requester_id = $2::uuid AND addressee_id = u.id) \
                    OR (addressee_id = $2::uuid AND requester_id = u.id)) \
             AND status = 'accepted') \
         ORDER BY u.username LIMIT 20",
    )
    .bind(auth.user_id.to_string())
    .bind(user_id.to_string())
    .fetch_all(&state.db.pool)
    .await?;

    let mutual_friends: Vec<serde_json::Value> = mutual_rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.try_get::<String, _>("id").unwrap_or_default(),
                "username": row.try_get::<String, _>("username").unwrap_or_default(),
                "display_name": row.try_get::<Option<String>, _>("display_name").unwrap_or(None),
                "avatar": row.try_get::<Option<String>, _>("avatar").unwrap_or(None),
            })
        })
        .collect();

    let resp = UserResponse::from(user);
    Ok(Json(serde_json::json!({
        "id": resp.id,
        "username": resp.username,
        "display_name": resp.display_name,
        "avatar": resp.avatar,
        "banner": resp.banner,
        "bio": resp.bio,
        "status": resp.status,
        "presence": resp.presence,
        "flags": resp.flags,
        "created_at": resp.created_at,
        "servers_in_common": servers_in_common,
        "mutual_friends": mutual_friends,
    })))
}
