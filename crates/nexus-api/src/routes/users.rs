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
