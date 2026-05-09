//! Server Discovery API (v0.16) — browse, search, and preview public servers.
//!
//! Extends the federated directory (v0.8) with local‑server discovery features:
//! tag / category filtering, full-text search, featured servers, and previews.
//!
//! ## Endpoints
//!
//! | Method | Path | Auth | Description |
//! |--------|------|------|-------------|
//! | GET  | `/api/v1/discover/servers`              | None   | Browse public servers (filter by tag/category) |
//! | GET  | `/api/v1/discover/servers/featured`      | None   | Featured server list |
//! | GET  | `/api/v1/discover/servers/search`        | None   | Full-text search |
//! | GET  | `/api/v1/discover/servers/{id}/preview`  | None   | Server preview (info + public channels) |
//! | POST | `/api/v1/discover/servers/{id}/feature`  | Bearer | Feature / un-feature (owner only) |
//! | GET  | `/api/v1/discover/categories`            | None   | List predefined categories |

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    middleware,
    routing::{get, post},
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::server::ServerResponse,
};
use nexus_db::repository::{channels, servers};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState, middleware::AuthContext};

// ─── Router ───────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Public (no auth)
        .route("/discover/servers", get(browse_servers))
        .route("/discover/servers/featured", get(featured_servers))
        .route("/discover/servers/search", get(search_servers))
        .route("/discover/servers/{server_id}/preview", get(server_preview))
        .route("/discover/categories", get(list_categories))
        // Authenticated
        .route(
            "/discover/servers/{server_id}/feature",
            post(toggle_feature).route_layer(middleware::from_fn(
                crate::middleware::combined_auth_middleware,
            )),
        )
}

// ─── Predefined categories ──────────────────────────────────────────────────

pub const CATEGORIES: &[&str] = &[
    "gaming",
    "education",
    "technology",
    "art",
    "music",
    "community",
    "science",
    "entertainment",
    "sports",
    "finance",
    "news",
    "other",
];

// ─── Request / Response types ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct BrowseQuery {
    tag: Option<String>,
    category: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
struct DiscoverList {
    servers: Vec<ServerResponse>,
    total: usize,
}

#[derive(Serialize)]
struct ChannelPreview {
    id: Uuid,
    name: Option<String>,
    channel_type: String,
    topic: Option<String>,
    position: i32,
}

#[derive(Serialize)]
struct ServerPreview {
    #[serde(flatten)]
    server: ServerResponse,
    channels: Vec<ChannelPreview>,
}

#[derive(Deserialize)]
struct FeatureRequest {
    featured: bool,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// `GET /api/v1/discover/servers?tag=...&category=...&limit=...&offset=...`
async fn browse_servers(
    State(state): State<Arc<AppState>>,
    Query(q): Query<BrowseQuery>,
) -> NexusResult<Json<DiscoverList>> {
    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);

    let servers = if let Some(ref tag) = q.tag {
        servers::list_servers_by_tag(&state.db.pool, tag, limit, offset).await?
    } else if let Some(ref cat) = q.category {
        servers::list_servers_by_category(&state.db.pool, cat, limit, offset).await?
    } else {
        servers::list_public_servers(&state.db.pool, limit, offset).await?
    };

    let total = servers.len();
    Ok(Json(DiscoverList {
        servers: servers.into_iter().map(Into::into).collect(),
        total,
    }))
}

/// `GET /api/v1/discover/servers/featured`
async fn featured_servers(State(state): State<Arc<AppState>>) -> NexusResult<Json<DiscoverList>> {
    let servers = servers::list_featured_servers(&state.db.pool, 20).await?;
    let total = servers.len();
    Ok(Json(DiscoverList {
        servers: servers.into_iter().map(Into::into).collect(),
        total,
    }))
}

/// `GET /api/v1/discover/servers/search?q=...&limit=...&offset=...`
async fn search_servers(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SearchQuery>,
) -> NexusResult<Json<DiscoverList>> {
    let query = q.q.unwrap_or_default();
    if query.is_empty() {
        return Ok(Json(DiscoverList {
            servers: vec![],
            total: 0,
        }));
    }
    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);

    let servers = servers::search_public_servers(&state.db.pool, &query, limit, offset).await?;
    let total = servers.len();
    Ok(Json(DiscoverList {
        servers: servers.into_iter().map(Into::into).collect(),
        total,
    }))
}

/// `GET /api/v1/discover/servers/:id/preview`
///
/// Returns server info + list of public channels (no messages).
async fn server_preview(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<ServerPreview>> {
    let server = servers::get_server_preview(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Server".into(),
        })?;

    let chans = channels::list_server_channels(&state.db.pool, server_id).await?;
    let previews: Vec<ChannelPreview> = chans
        .into_iter()
        .map(|c| ChannelPreview {
            id: c.id,
            name: c.name,
            channel_type: format!("{:?}", c.channel_type).to_lowercase(),
            topic: c.topic,
            position: c.position,
        })
        .collect();

    Ok(Json(ServerPreview {
        server: server.into(),
        channels: previews,
    }))
}

/// `POST /api/v1/discover/servers/:id/feature` — Feature or un-feature a server.
///
/// Only the server owner can feature their own server.
async fn toggle_feature(
    axum::Extension(auth): axum::Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<FeatureRequest>,
) -> NexusResult<Json<ServerResponse>> {
    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Server".into(),
        })?;

    if server.owner_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }
    if !server.is_public {
        return Err(NexusError::Validation {
            message: "Only public servers can be featured".into(),
        });
    }

    let updated = servers::set_featured(&state.db.pool, server_id, body.featured).await?;
    Ok(Json(updated.into()))
}

/// `GET /api/v1/discover/categories`
async fn list_categories() -> Json<Vec<&'static str>> {
    Json(CATEGORIES.to_vec())
}
