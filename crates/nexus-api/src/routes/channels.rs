//! Channel routes — CRUD for channels within a server.

use axum::{
    extract::{Extension, Path, State},
    routing::get,
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::channel::{CreateChannelRequest, UpdateChannelRequest},
    snowflake,
    validation::validate_request,
};
use nexus_db::repository::{channels, servers};
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

/// Channel routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/servers/{server_id}/channels",
            get(list_channels).post(create_channel),
        )
        .route(
            "/channels/{channel_id}",
            get(get_channel).patch(update_channel).delete(delete_channel),
        )
}

/// GET /api/v1/servers/:server_id/channels
async fn list_channels(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<nexus_common::models::channel::Channel>>> {
    let channel_list = channels::list_server_channels(&state.db.pg, server_id).await?;
    Ok(Json(channel_list))
}

/// POST /api/v1/servers/:server_id/channels
async fn create_channel(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateChannelRequest>,
) -> NexusResult<Json<nexus_common::models::channel::Channel>> {
    validate_request(&body)?;

    // Verify server exists and user has permission
    let server = servers::find_by_id(&state.db.pg, server_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Server".into(),
        })?;

    // For now, only owner can create channels (TODO: proper permission check)
    if server.owner_id != auth.user_id {
        return Err(NexusError::MissingPermission {
            permission: "MANAGE_CHANNELS".into(),
        });
    }

    let channel_id = snowflake::generate_id();
    let channel_type_str = serde_json::to_value(&body.channel_type)
        .map_err(|e| NexusError::Internal(e.into()))?
        .as_str()
        .unwrap_or("text")
        .to_string();

    let channel = channels::create_channel(
        &state.db.pg,
        channel_id,
        Some(server_id),
        body.parent_id,
        &channel_type_str,
        Some(&body.name),
        body.topic.as_deref(),
        body.position.unwrap_or(0),
    )
    .await?;

    tracing::info!(
        channel_id = %channel_id,
        server_id = %server_id,
        name = %body.name,
        "Channel created"
    );

    Ok(Json(channel))
}

/// GET /api/v1/channels/:channel_id
async fn get_channel(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<nexus_common::models::channel::Channel>> {
    let channel = channels::find_by_id(&state.db.pg, channel_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Channel".into(),
        })?;

    Ok(Json(channel))
}

/// PATCH /api/v1/channels/:channel_id
async fn update_channel(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<UpdateChannelRequest>,
) -> NexusResult<Json<nexus_common::models::channel::Channel>> {
    validate_request(&body)?;

    let _channel = channels::find_by_id(&state.db.pg, channel_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Channel".into(),
        })?;

    // TODO: proper permission check
    let updated = channels::update_channel(
        &state.db.pg,
        channel_id,
        body.name.as_deref(),
        body.topic.as_deref(),
        body.position,
        body.nsfw,
        body.rate_limit_per_user,
    )
    .await?;

    Ok(Json(updated))
}

/// DELETE /api/v1/channels/:channel_id
async fn delete_channel(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    // TODO: proper permission check
    channels::delete_channel(&state.db.pg, channel_id).await?;

    tracing::info!(channel_id = %channel_id, "Channel deleted");

    Ok(Json(serde_json::json!({ "deleted": true })))
}
