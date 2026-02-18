//! Slash command routes — register, list, delete application commands.
//!
//! Both user-authenticated (developer portal) and bot-authenticated flows are
//! supported. The bot middleware sets a `BotContext` extension for bot-token
//! requests. For user requests, the `AuthContext` extension is set.

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::GatewayEvent,
    models::slash_command::{
        CreateInteractionRequest, Interaction, InteractionResponse, SlashCommand,
        UpsertCommandRequest,
    },
    snowflake,
};
use nexus_db::repository::{bots, slash_commands};
use rand::Rng;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

/// Slash command routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Global commands (developer portal or bot token)
        .route(
            "/applications/{app_id}/commands",
            get(get_global_commands).put(bulk_overwrite_global_commands),
        )
        .route(
            "/applications/{app_id}/commands/{command_id}",
            get(get_global_command)
                .patch(edit_global_command)
                .delete(delete_global_command),
        )
        .route("/applications/{app_id}/commands", post(create_global_command))
        // Server-scoped commands
        .route(
            "/applications/{app_id}/guilds/{server_id}/commands",
            get(get_server_commands)
                .post(create_server_command)
                .put(bulk_overwrite_server_commands),
        )
        .route(
            "/applications/{app_id}/guilds/{server_id}/commands/{command_id}",
            get(get_server_command)
                .patch(edit_server_command)
                .delete(delete_server_command),
        )
        // Client-facing: get commands available in a server (for the slash menu)
        .route("/servers/{server_id}/commands", get(list_available_commands))
        // Interactions (client → server → bot pipeline)
        .route("/interactions", post(create_interaction))
        .route(
            "/interactions/{interaction_id}/callback",
            post(interaction_callback),
        )
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

// ============================================================================
// Global Commands
// ============================================================================

/// GET /api/v1/applications/{app_id}/commands
async fn get_global_commands(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<Uuid>,
) -> NexusResult<Json<Vec<SlashCommand>>> {
    verify_app_access(&state, app_id, auth.user_id).await?;
    let cmds = slash_commands::get_global_commands(&state.db.pg, app_id).await?;
    Ok(Json(cmds))
}

/// GET /api/v1/applications/{app_id}/commands/{command_id}
async fn get_global_command(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((app_id, command_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<SlashCommand>> {
    verify_app_access(&state, app_id, auth.user_id).await?;
    let cmd = slash_commands::get_command(&state.db.pg, command_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "command".to_string() })?;
    Ok(Json(cmd))
}

/// POST /api/v1/applications/{app_id}/commands — Create a global command.
async fn create_global_command(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<Uuid>,
    Json(body): Json<UpsertCommandRequest>,
) -> NexusResult<Json<SlashCommand>> {
    verify_app_access(&state, app_id, auth.user_id).await?;
    let id = snowflake::generate_id();
    let cmd = slash_commands::upsert_command(
        &state.db.pg,
        id,
        app_id,
        None,
        &body.name,
        &body.description,
        &body.options.unwrap_or_default(),
        body.command_type.unwrap_or(1),
        body.default_member_permissions.as_deref(),
        body.dm_permission.unwrap_or(true),
    )
    .await?;

    broadcast_command_event(&state, &cmd, "APPLICATION_COMMAND_CREATE");
    Ok(Json(cmd))
}

/// PATCH /api/v1/applications/{app_id}/commands/{command_id}
async fn edit_global_command(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((app_id, command_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpsertCommandRequest>,
) -> NexusResult<Json<SlashCommand>> {
    verify_app_access(&state, app_id, auth.user_id).await?;
    let existing = slash_commands::get_command(&state.db.pg, command_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "command".to_string() })?;

    let cmd = slash_commands::upsert_command(
        &state.db.pg,
        command_id,
        app_id,
        existing.server_id,
        &body.name,
        &body.description,
        &body.options.unwrap_or(existing.options),
        body.command_type.unwrap_or(existing.command_type),
        body.default_member_permissions
            .as_deref()
            .or(existing.default_member_permissions.as_deref()),
        body.dm_permission.unwrap_or(existing.dm_permission),
    )
    .await?;

    broadcast_command_event(&state, &cmd, "APPLICATION_COMMAND_UPDATE");
    Ok(Json(cmd))
}

/// DELETE /api/v1/applications/{app_id}/commands/{command_id}
async fn delete_global_command(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((app_id, command_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<axum::http::StatusCode> {
    verify_app_access(&state, app_id, auth.user_id).await?;
    slash_commands::delete_command(&state.db.pg, command_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// PUT /api/v1/applications/{app_id}/commands — Bulk overwrite global commands.
async fn bulk_overwrite_global_commands(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<Uuid>,
    Json(body): Json<Vec<UpsertCommandRequest>>,
) -> NexusResult<Json<Vec<SlashCommand>>> {
    verify_app_access(&state, app_id, auth.user_id).await?;

    // Build the commands tuple array expected by the repo
    let commands: Vec<(Uuid, String, String, serde_json::Value, i32)> = body
        .iter()
        .map(|req| {
            let id = snowflake::generate_id();
            let opts =
                serde_json::to_value(req.options.as_deref().unwrap_or(&[])).unwrap_or_default();
            (id, req.name.clone(), req.description.clone(), opts, req.command_type.unwrap_or(1))
        })
        .collect();

    let cmds =
        slash_commands::bulk_overwrite_global_commands(&state.db.pg, app_id, &commands).await?;
    Ok(Json(cmds))
}

// ============================================================================
// Server-Scoped Commands
// ============================================================================

/// GET /api/v1/applications/{app_id}/guilds/{server_id}/commands
async fn get_server_commands(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((app_id, server_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<Vec<SlashCommand>>> {
    verify_app_access(&state, app_id, auth.user_id).await?;
    let cmds = slash_commands::get_server_commands(&state.db.pg, app_id, server_id).await?;
    Ok(Json(cmds))
}

/// GET /api/v1/applications/{app_id}/guilds/{server_id}/commands/{command_id}
async fn get_server_command(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((app_id, _server_id, command_id)): Path<(Uuid, Uuid, Uuid)>,
) -> NexusResult<Json<SlashCommand>> {
    verify_app_access(&state, app_id, auth.user_id).await?;
    let cmd = slash_commands::get_command(&state.db.pg, command_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "command".to_string() })?;
    Ok(Json(cmd))
}

/// POST /api/v1/applications/{app_id}/guilds/{server_id}/commands
async fn create_server_command(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((app_id, server_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpsertCommandRequest>,
) -> NexusResult<Json<SlashCommand>> {
    verify_app_access(&state, app_id, auth.user_id).await?;
    let id = snowflake::generate_id();
    let cmd = slash_commands::upsert_command(
        &state.db.pg,
        id,
        app_id,
        Some(server_id),
        &body.name,
        &body.description,
        &body.options.unwrap_or_default(),
        body.command_type.unwrap_or(1),
        body.default_member_permissions.as_deref(),
        body.dm_permission.unwrap_or(true),
    )
    .await?;
    broadcast_command_event(&state, &cmd, "APPLICATION_COMMAND_CREATE");
    Ok(Json(cmd))
}

/// PATCH /api/v1/applications/{app_id}/guilds/{server_id}/commands/{command_id}
async fn edit_server_command(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((app_id, _server_id, command_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpsertCommandRequest>,
) -> NexusResult<Json<SlashCommand>> {
    verify_app_access(&state, app_id, auth.user_id).await?;
    let existing = slash_commands::get_command(&state.db.pg, command_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "command".to_string() })?;
    let cmd = slash_commands::upsert_command(
        &state.db.pg,
        command_id,
        app_id,
        existing.server_id,
        &body.name,
        &body.description,
        &body.options.unwrap_or(existing.options),
        body.command_type.unwrap_or(existing.command_type),
        body.default_member_permissions
            .as_deref()
            .or(existing.default_member_permissions.as_deref()),
        body.dm_permission.unwrap_or(existing.dm_permission),
    )
    .await?;
    broadcast_command_event(&state, &cmd, "APPLICATION_COMMAND_UPDATE");
    Ok(Json(cmd))
}

/// DELETE /api/v1/applications/{app_id}/guilds/{server_id}/commands/{command_id}
async fn delete_server_command(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((app_id, _server_id, command_id)): Path<(Uuid, Uuid, Uuid)>,
) -> NexusResult<axum::http::StatusCode> {
    verify_app_access(&state, app_id, auth.user_id).await?;
    slash_commands::delete_command(&state.db.pg, command_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// PUT /api/v1/applications/{app_id}/guilds/{server_id}/commands — Bulk overwrite.
async fn bulk_overwrite_server_commands(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((app_id, server_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<Vec<UpsertCommandRequest>>,
) -> NexusResult<Json<Vec<SlashCommand>>> {
    verify_app_access(&state, app_id, auth.user_id).await?;

    // Build the commands tuple array expected by the repo
    let commands: Vec<(Uuid, String, String, serde_json::Value, i32)> = body
        .iter()
        .map(|req| {
            let id = snowflake::generate_id();
            let opts =
                serde_json::to_value(req.options.as_deref().unwrap_or(&[])).unwrap_or_default();
            (id, req.name.clone(), req.description.clone(), opts, req.command_type.unwrap_or(1))
        })
        .collect();

    let cmds = slash_commands::bulk_overwrite_server_commands(
        &state.db.pg,
        app_id,
        server_id,
        &commands,
    )
    .await?;
    Ok(Json(cmds))
}

// ============================================================================
// Client-Facing: Available Commands in a Server
// ============================================================================

#[derive(Deserialize)]
struct AvailableCommandsQuery {
    application_id: Option<Uuid>,
}

/// GET /api/v1/servers/{server_id}/commands — List all commands the user can invoke.
async fn list_available_commands(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<AvailableCommandsQuery>,
) -> NexusResult<Json<Vec<SlashCommand>>> {
    let cmds = if let Some(app_id) = q.application_id {
        slash_commands::get_server_commands(&state.db.pg, app_id, server_id).await?
    } else {
        slash_commands::get_all_server_commands(&state.db.pg, server_id).await?
    };
    Ok(Json(cmds))
}

// ============================================================================
// Interaction Pipeline
// ============================================================================

/// POST /api/v1/interactions — Client submits a slash command invocation.
async fn create_interaction(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateInteractionRequest>,
) -> NexusResult<Json<Interaction>> {
    // Resolve command to find application_id
    let command_id = body.command_id;
    let app_id = if let Some(cid) = command_id {
        slash_commands::get_command(&state.db.pg, cid)
            .await?
            .ok_or(NexusError::NotFound { resource: "command".to_string() })?
            .application_id
    } else {
        return Err(NexusError::Validation {
            message: "command_id is required for APPLICATION_COMMAND interactions".into(),
        });
    };

    let interaction_id = snowflake::generate_id();
    // Interaction token is a one-time secret for the bot to respond with
    let token: String = rand::rng()
        .sample_iter(rand::distr::Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    let interaction = slash_commands::create_interaction(
        &state.db.pg,
        interaction_id,
        app_id,
        &body.interaction_type,
        Some(body.data.clone()),
        None, // server_id — should come from client context
        None, // channel_id — should come from client context
        auth.user_id,
        &token,
    )
    .await?;

    // Emit INTERACTION_CREATE to the gateway so the bot can pick it up
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: nexus_common::gateway_event::event_types::INTERACTION_CREATE.to_string(),
        data: serde_json::to_value(&interaction).unwrap_or_default(),
        server_id: interaction.server_id,
        channel_id: interaction.channel_id,
        user_id: Some(auth.user_id),
    });

    Ok(Json(interaction))
}

/// POST /api/v1/interactions/{interaction_id}/callback — Bot responds to interaction.
async fn interaction_callback(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(_interaction_id): Path<Uuid>,
    Json(body): Json<InteractionResponse>,
) -> NexusResult<axum::http::StatusCode> {
    // Mark interaction as responded
    // (update_interaction_status not yet implemented in repo — skipped)

    // If the response includes message data, broadcast it
    if body.response_type == 4 || body.response_type == 7 {
        if let Some(data) = &body.data {
            let _ = state.gateway_tx.send(GatewayEvent {
                event_type: nexus_common::gateway_event::event_types::MESSAGE_CREATE.to_string(),
                data: data.clone(),
                server_id: None,
                channel_id: None,
                user_id: None,
            });
        }
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ============================================================================
// Helpers
// ============================================================================

async fn verify_app_access(
    state: &AppState,
    app_id: Uuid,
    user_id: Uuid,
) -> NexusResult<()> {
    let app = bots::get_bot(&state.db.pg, app_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "application".to_string() })?;
    if app.owner_id != user_id {
        return Err(NexusError::Forbidden);
    }
    Ok(())
}

fn broadcast_command_event(state: &AppState, cmd: &SlashCommand, event_type: &str) {
    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_type.to_string(),
        data: serde_json::to_value(cmd).unwrap_or_default(),
        server_id: cmd.server_id,
        channel_id: None,
        user_id: None,
    });
}
