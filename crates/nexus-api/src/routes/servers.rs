//! Server (guild) routes — create, join, leave, manage.

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::server::{CreateServerRequest, ServerResponse, UpdateServerRequest},
    permissions::Permissions,
    snowflake,
    validation::validate_request,
};
use nexus_db::repository::{channels, members, roles, servers};
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

/// Server routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/servers", get(list_my_servers).post(create_server))
        .route("/servers/{server_id}", get(get_server).patch(update_server).delete(delete_server))
        .route("/servers/{server_id}/members", get(list_members))
        .route("/servers/{server_id}/join", post(join_server))
        .route("/servers/{server_id}/leave", post(leave_server))
        .route("/servers/{server_id}/invites", post(create_invite_route))
        .route("/invites/{code}", get(get_invite_route))
        .route("/invites/{code}/join", post(join_via_invite_route))
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

/// Generate a short random alphanumeric invite code.
fn generate_invite_code() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..36u8);
            (if idx < 10 { b'0' + idx } else { b'a' + idx - 10 }) as char
        })
        .collect()
}

/// GET /api/v1/servers — List servers the authenticated user is a member of.
async fn list_my_servers(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<Vec<ServerResponse>>> {
    let user_servers = servers::list_user_servers(&state.db.pool, auth.user_id).await?;
    Ok(Json(user_servers.into_iter().map(|s| s.into()).collect()))
}

/// POST /api/v1/servers — Create a new server.
async fn create_server(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateServerRequest>,
) -> NexusResult<Json<ServerResponse>> {
    validate_request(&body)?;

    let config = nexus_common::config::get();

    // Check server limit
    let user_servers = servers::list_user_servers(&state.db.pool, auth.user_id).await?;
    if user_servers.len() >= config.limits.max_servers_per_user as usize {
        return Err(NexusError::LimitReached {
            message: format!(
                "You can be in at most {} servers",
                config.limits.max_servers_per_user
            ),
        });
    }

    let server_id = snowflake::generate_id();
    let is_public = body.is_public.unwrap_or(false);

    // Create the server
    let server =
        servers::create_server(&state.db.pool, server_id, &body.name, auth.user_id, is_public)
            .await?;

    // Create @everyone role with default permissions
    let everyone_role_id = snowflake::generate_id();
    roles::create_role(
        &state.db.pool,
        everyone_role_id,
        server_id,
        "@everyone",
        None,
        Permissions::default_everyone().bits(),
        0,
        true,
    )
    .await?;

    // Create default channels
    let general_id = snowflake::generate_id();
    channels::create_channel(
        &state.db.pool,
        general_id,
        Some(server_id),
        None,
        "text",
        Some("general"),
        Some("General discussion"),
        0,
    )
    .await?;

    let voice_id = snowflake::generate_id();
    channels::create_channel(
        &state.db.pool,
        voice_id,
        Some(server_id),
        None,
        "voice",
        Some("General"),
        None,
        1,
    )
    .await?;

    // Add creator as member
    members::add_member(&state.db.pool, auth.user_id, server_id).await?;

    tracing::info!(
        server_id = %server_id,
        owner = %auth.user_id,
        name = %body.name,
        "Server created"
    );

    Ok(Json(server.into()))
}

/// GET /api/v1/servers/:server_id
async fn get_server(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<ServerResponse>> {
    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Server".into(),
        })?;

    Ok(Json(server.into()))
}

/// PATCH /api/v1/servers/:server_id
async fn update_server(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<UpdateServerRequest>,
) -> NexusResult<Json<ServerResponse>> {
    validate_request(&body)?;

    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Server".into(),
        })?;

    // Only owner or admin can update
    if server.owner_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }

    let updated = servers::update_server(
        &state.db.pool,
        server_id,
        body.name.as_deref(),
        body.description.as_deref(),
        body.is_public,
    )
    .await?;

    Ok(Json(updated.into()))
}

/// DELETE /api/v1/servers/:server_id
async fn delete_server(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Server".into(),
        })?;

    if server.owner_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }

    servers::delete_server(&state.db.pool, server_id).await?;

    tracing::info!(server_id = %server_id, "Server deleted");

    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// GET /api/v1/servers/:server_id/members
async fn list_members(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<nexus_common::models::member::MemberResponse>>> {
    let members_list = members::list_members(&state.db.pool, server_id, 1000, 0).await?;
    Ok(Json(members_list.into_iter().map(Into::into).collect()))
}

/// POST /api/v1/servers/:server_id/join
async fn join_server(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    // Check server exists and is public (or user has invite)
    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Server".into(),
        })?;

    if !server.is_public {
        return Err(NexusError::Forbidden);
    }

    // Check if already a member
    if members::is_member(&state.db.pool, auth.user_id, server_id).await? {
        return Err(NexusError::AlreadyExists {
            resource: "Membership".into(),
        });
    }

    members::add_member(&state.db.pool, auth.user_id, server_id).await?;
    servers::increment_member_count(&state.db.pool, server_id).await?;

    Ok(Json(serde_json::json!({ "joined": true })))
}

/// POST /api/v1/servers/:server_id/leave
async fn leave_server(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Server".into(),
        })?;

    // Owner can't leave (must transfer or delete)
    if server.owner_id == auth.user_id {
        return Err(NexusError::Validation {
            message: "Server owner cannot leave. Transfer ownership or delete the server.".into(),
        });
    }

    members::remove_member(&state.db.pool, auth.user_id, server_id).await?;
    servers::decrement_member_count(&state.db.pool, server_id).await?;

    Ok(Json(serde_json::json!({ "left": true })))
}
/// POST /api/v1/servers/:server_id/invites
async fn create_invite_route(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<nexus_common::models::server::CreateInviteRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    // Must be a member to create an invite
    if !members::is_member(&state.db.pool, auth.user_id, server_id).await? {
        return Err(NexusError::Forbidden);
    }

    let code = generate_invite_code();
    let expires_at = body.max_age_secs.filter(|&s| s > 0).map(|s| {
        chrono::Utc::now() + chrono::Duration::seconds(s as i64)
    });
    let max_uses = body.max_uses.filter(|&u| u > 0);

    let invite = servers::create_invite(
        &state.db.pool,
        &code,
        server_id,
        None,
        auth.user_id,
        max_uses,
        expires_at,
    )
    .await?;

    Ok(Json(serde_json::json!({ "code": invite.code })))
}

/// GET /api/v1/invites/:code — public, no auth required
async fn get_invite_route(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> NexusResult<Json<serde_json::Value>> {
    let invite = servers::find_invite(&state.db.pool, &code)
        .await?
        .ok_or(NexusError::NotFound { resource: "Invite".into() })?;

    if let Some(exp) = invite.expires_at {
        if exp < chrono::Utc::now() {
            return Err(NexusError::NotFound { resource: "Invite".into() });
        }
    }
    if let Some(max) = invite.max_uses {
        if invite.uses >= max {
            return Err(NexusError::NotFound { resource: "Invite".into() });
        }
    }

    let server = servers::find_by_id(&state.db.pool, invite.server_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Server".into() })?;

    Ok(Json(serde_json::json!({
        "code": invite.code,
        "server": { "id": server.id, "name": server.name, "member_count": server.member_count },
        "uses": invite.uses,
        "max_uses": invite.max_uses,
    })))
}

/// POST /api/v1/invites/:code/join
async fn join_via_invite_route(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> NexusResult<Json<serde_json::Value>> {
    let invite = servers::find_invite(&state.db.pool, &code)
        .await?
        .ok_or(NexusError::NotFound { resource: "Invite".into() })?;

    if let Some(exp) = invite.expires_at {
        if exp < chrono::Utc::now() {
            return Err(NexusError::Validation { message: "Invite has expired.".into() });
        }
    }
    if let Some(max) = invite.max_uses {
        if invite.uses >= max {
            return Err(NexusError::Validation { message: "Invite has reached its maximum uses.".into() });
        }
    }

    let server_id = invite.server_id;

    // Already a member — just return the server info
    if members::is_member(&state.db.pool, auth.user_id, server_id).await? {
        let server = servers::find_by_id(&state.db.pool, server_id)
            .await?
            .ok_or(NexusError::NotFound { resource: "Server".into() })?;
        return Ok(Json(serde_json::json!({
            "server": { "id": server.id, "name": server.name }
        })));
    }

    members::add_member(&state.db.pool, auth.user_id, server_id).await?;
    servers::use_invite(&state.db.pool, &code).await?;
    servers::increment_member_count(&state.db.pool, server_id).await?;

    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Server".into() })?;

    Ok(Json(serde_json::json!({
        "server": { "id": server.id, "name": server.name }
    })))
}