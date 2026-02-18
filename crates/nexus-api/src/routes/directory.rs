//! Public server directory and federated room discovery (v0.8).
//!
//! The directory allows users to discover public servers and rooms across the
//! federated Nexus network.
//!
//! ## Endpoints
//!
//! | Method | Path | Auth | Description |
//! |--------|------|------|-------------|
//! | GET  | `/api/v1/directory/servers` | None | List all known public servers |
//! | GET  | `/api/v1/directory/rooms` | None | List all public rooms (federated) |
//! | GET  | `/api/v1/directory/rooms/search` | None | Search rooms by name/topic |
//! | POST | `/api/v1/directory/rooms/join` | Bearer | Join a federated room |
//! | GET  | `/api/v1/directory/resolve/:server_name` | None | Resolve a server's base URL |

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

use crate::AppState;

// ─── Router ───────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Public listing (no auth)
        .route("/directory/servers", get(list_servers))
        .route("/directory/rooms", get(list_rooms))
        .route("/directory/rooms/search", get(search_rooms))
        .route("/directory/resolve/:server_name", get(resolve_server))
        // Authenticated actions
        .route(
            "/directory/rooms/join",
            post(join_federated_room)
                .route_layer(middleware::from_fn(crate::middleware::auth_middleware)),
        )
}

// ─── Request / response types ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct PaginationQuery {
    limit: Option<u32>,
    since: Option<String>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    server: Option<String>,
    limit: Option<u32>,
    since: Option<String>,
}

#[derive(Deserialize)]
struct JoinRoomRequest {
    /// Fully-qualified room ID (`!id:server.tld`) or alias (`#alias:server.tld`).
    room_id: String,
}

#[derive(Serialize)]
struct ServerEntry {
    server_name: String,
    description: Option<String>,
    icon_url: Option<String>,
    public_room_count: u64,
    total_users: u64,
}

#[derive(Serialize)]
struct RoomEntry {
    room_id: String,
    name: Option<String>,
    topic: Option<String>,
    member_count: u64,
    server_name: String,
    join_rule: String,
    tags: Vec<String>,
}

#[derive(Serialize)]
struct PaginatedRooms {
    rooms: Vec<RoomEntry>,
    total_count: u64,
    next_batch: Option<String>,
}

#[derive(Serialize)]
struct PaginatedServers {
    servers: Vec<ServerEntry>,
    total_count: u64,
    next_batch: Option<String>,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// `GET /api/v1/directory/servers`
///
/// Return all servers listed in the `directory_servers` table.
/// This includes our own server plus any federated servers that have opted in.
async fn list_servers(
    State(_state): State<Arc<AppState>>,
    Query(q): Query<PaginationQuery>,
) -> Json<PaginatedServers> {
    let limit = q.limit.unwrap_or(20).min(100) as i64;

    // TODO: query directory_servers table from DB.
    // Placeholder response so the endpoint works immediately.
    let this_server =
        std::env::var("NEXUS_SERVER_NAME").unwrap_or_else(|_| "localhost".to_owned());

    Json(PaginatedServers {
        servers: vec![ServerEntry {
            server_name: this_server,
            description: Some("This Nexus server".to_owned()),
            icon_url: None,
            public_room_count: 0,
            total_users: 0,
        }],
        total_count: 1,
        next_batch: None,
    })
}

/// `GET /api/v1/directory/rooms`
///
/// Return all publicly joinable federated rooms — from this server and any
/// remote servers in the directory.
async fn list_rooms(
    State(_state): State<Arc<AppState>>,
    Query(q): Query<PaginationQuery>,
) -> Json<PaginatedRooms> {
    let limit = q.limit.unwrap_or(20).min(100) as i64;
    // TODO: query federated_rooms WHERE join_rule = 'public' ORDER BY member_count DESC.
    Json(PaginatedRooms { rooms: vec![], total_count: 0, next_batch: None })
}

/// `GET /api/v1/directory/rooms/search?q=<query>&server=<server>&limit=<n>`
///
/// Full-text search across public room names and topics, optionally scoped
/// to a specific server.
async fn search_rooms(
    State(_state): State<Arc<AppState>>,
    Query(q): Query<SearchQuery>,
) -> Json<PaginatedRooms> {
    let _query_str = q.q.unwrap_or_default();
    let _server_filter = q.server;
    let _limit = q.limit.unwrap_or(20).min(100) as i64;

    // TODO: use MeiliSearch or ILIKE across federated_rooms to return matches.
    Json(PaginatedRooms { rooms: vec![], total_count: 0, next_batch: None })
}

/// `GET /api/v1/directory/resolve/:server_name`
///
/// Return the resolved federation base URL and key information for a server.
/// Useful for clients that want to verify a server before joining a room.
async fn resolve_server(
    State(_state): State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Json<Value> {
    // TODO: call DiscoveryCache::resolve and return the result + cached key info.
    Json(json!({
        "server_name": server_name,
        "base_url": format!("https://{}:8448", server_name),
        "status": "unknown",
    }))
}

/// `POST /api/v1/directory/rooms/join`
///
/// Initiate a federated join on behalf of the authenticated user.
/// If the room is on a remote server, this triggers the make_join → send_join
/// federation protocol.
async fn join_federated_room(
    State(_state): State<Arc<AppState>>,
    axum::extract::Extension(_auth): axum::extract::Extension<crate::middleware::AuthContext>,
    Json(body): Json<JoinRoomRequest>,
) -> (StatusCode, Json<Value>) {
    let room_id = body.room_id;
    info!("Federated join request for room {}", room_id);

    // TODO: parse room_id server part, resolve server, make_join, send_join.
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "message": "Federated join initiated",
            "room_id": room_id,
            "status": "pending",
        })),
    )
}
