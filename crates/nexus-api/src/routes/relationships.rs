//! Relationship routes — friends, pending requests, and blocks.
//!
//! Endpoints:
//!   GET    /users/@me/relationships             — list all relationships
//!   POST   /users/@me/relationships             — send friend request by username
//!   PATCH  /users/@me/relationships/:user_id    — accept / deny / block
//!   DELETE /users/@me/relationships/:user_id    — remove friend / cancel / unblock
//!   GET    /users/search?q=<username>           — find users by username prefix

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::get,
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::relationship::RelationshipStatus,
    snowflake,
};
use nexus_db::repository::{relationships, users};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/users/@me/relationships",
            get(list_relationships).post(send_friend_request),
        )
        .route(
            "/users/@me/relationships/{user_id}",
            axum::routing::patch(update_relationship)
                .delete(delete_relationship),
        )
        .route("/users/search", get(search_users))
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

// ── Request/Response shapes ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SendFriendRequest {
    /// Exact username of the user to add.
    username: String,
}

#[derive(Debug, Deserialize)]
struct UpdateRelationshipRequest {
    /// "accept" | "deny" | "block"
    action: String,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Debug, Serialize)]
struct RelationshipResponse {
    id: Uuid,
    /// Which direction: "outgoing" (I sent) or "incoming" (they sent).
    direction: String,
    status: String,
    user: UserBrief,
}

#[derive(Debug, Serialize)]
struct UserBrief {
    id: Uuid,
    username: String,
    display_name: Option<String>,
    avatar: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/v1/users/@me/relationships
async fn list_relationships(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<Vec<RelationshipResponse>>> {
    let rows = relationships::list_for_user(&state.db.pool, auth.user_id).await?;

    let mut result = Vec::with_capacity(rows.len());
    for rel in rows {
        let other_id = if rel.requester_id == auth.user_id {
            rel.addressee_id
        } else {
            rel.requester_id
        };
        let direction = if rel.requester_id == auth.user_id {
            "outgoing".to_string()
        } else {
            "incoming".to_string()
        };

        let status_str = match rel.status {
            RelationshipStatus::Pending => "pending",
            RelationshipStatus::Accepted => "accepted",
            RelationshipStatus::Blocked => "blocked",
        };

        if let Some(other) = users::find_by_id(&state.db.pool, other_id).await? {
            result.push(RelationshipResponse {
                id: other.id,
                direction,
                status: status_str.to_string(),
                user: UserBrief {
                    id: other.id,
                    username: other.username,
                    display_name: other.display_name,
                    avatar: other.avatar,
                },
            });
        }
    }

    Ok(Json(result))
}

/// POST /api/v1/users/@me/relationships — send a friend request by username.
async fn send_friend_request(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<SendFriendRequest>,
) -> NexusResult<Json<RelationshipResponse>> {
    // Look up target user
    let target = users::find_by_username(&state.db.pool, &body.username)
        .await?
        .ok_or(NexusError::NotFound { resource: "User".into() })?;

    if target.id == auth.user_id {
        return Err(NexusError::Validation {
            message: "Cannot add yourself as a friend".into(),
        });
    }

    // Check for existing relationship
    if let Some(_existing) = relationships::find_between(&state.db.pool, auth.user_id, target.id).await? {
        return Err(NexusError::Validation {
            message: "A relationship with this user already exists".into(),
        });
    }

    let rel_id = snowflake::generate_id();
    let _rel = relationships::create(
        &state.db.pool,
        rel_id,
        auth.user_id,
        target.id,
        "pending",
    )
    .await?;

    Ok(Json(RelationshipResponse {
        id: target.id,
        direction: "outgoing".to_string(),
        status: "pending".to_string(),
        user: UserBrief {
            id: target.id,
            username: target.username.clone(),
            display_name: target.display_name.clone(),
            avatar: target.avatar.clone(),
        },
    }))
}

/// PATCH /api/v1/users/@me/relationships/:user_id — accept / deny / block.
async fn update_relationship(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(other_user_id): Path<Uuid>,
    Json(body): Json<UpdateRelationshipRequest>,
) -> NexusResult<Json<RelationshipResponse>> {
    let rel = relationships::find_between(&state.db.pool, auth.user_id, other_user_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Relationship".into() })?;

    let new_status = match body.action.as_str() {
        "accept" => {
            // Only the addressee can accept
            if rel.addressee_id != auth.user_id {
                return Err(NexusError::Forbidden);
            }
            "accepted"
        }
        "deny" => {
            // Only the addressee can deny
            if rel.addressee_id != auth.user_id {
                return Err(NexusError::Forbidden);
            }
            // Delete rather than update status
            relationships::delete(&state.db.pool, rel.id).await?;
            let other = users::find_by_id(&state.db.pool, other_user_id)
                .await?
                .ok_or(NexusError::NotFound { resource: "User".into() })?;
            return Ok(Json(RelationshipResponse {
                id: other.id,
                direction: "incoming".to_string(),
                status: "denied".to_string(),
                user: UserBrief {
                    id: other.id,
                    username: other.username,
                    display_name: other.display_name,
                    avatar: other.avatar,
                },
            }));
        }
        "block" => "blocked",
        _ => {
            return Err(NexusError::Validation {
                message: "action must be 'accept', 'deny', or 'block'".into(),
            });
        }
    };

    let updated = relationships::update_status(&state.db.pool, rel.id, new_status).await?;

    let other = users::find_by_id(&state.db.pool, other_user_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "User".into() })?;

    let direction = if updated.requester_id == auth.user_id {
        "outgoing"
    } else {
        "incoming"
    };

    Ok(Json(RelationshipResponse {
        id: other.id,
        direction: direction.to_string(),
        status: new_status.to_string(),
        user: UserBrief {
            id: other.id,
            username: other.username,
            display_name: other.display_name,
            avatar: other.avatar,
        },
    }))
}

/// DELETE /api/v1/users/@me/relationships/:user_id — remove friend / cancel / unblock.
async fn delete_relationship(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(other_user_id): Path<Uuid>,
) -> NexusResult<axum::http::StatusCode> {
    let rel = relationships::find_between(&state.db.pool, auth.user_id, other_user_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Relationship".into() })?;

    relationships::delete(&state.db.pool, rel.id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// GET /api/v1/users/search?q=<prefix> — search users by username prefix.
async fn search_users(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> NexusResult<Json<Vec<UserBrief>>> {
    if params.q.len() < 2 {
        return Err(NexusError::Validation {
            message: "Search query must be at least 2 characters".into(),
        });
    }

    let q = format!(
        "SELECT {cols} FROM users \
         WHERE LOWER(username) LIKE LOWER($1) AND id != $2::uuid \
         ORDER BY username \
         LIMIT 20",
        cols = nexus_db::select_cols::USER_COLS
    );
    let like_pattern = format!("{}%", params.q.to_lowercase());
    let found = sqlx::query_as::<_, nexus_common::models::user::User>(&q)
        .bind(like_pattern)
        .bind(auth.user_id.to_string())
        .fetch_all(&state.db.pool)
        .await?;

    let briefs: Vec<UserBrief> = found
        .into_iter()
        .map(|u| UserBrief {
            id: u.id,
            username: u.username,
            display_name: u.display_name,
            avatar: u.avatar,
        })
        .collect();

    Ok(Json(briefs))
}
