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
use sqlx::Row as _;
use tracing::{info, warn};

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
    State(state): State<Arc<AppState>>,
    Query(q): Query<PaginationQuery>,
) -> Json<PaginatedServers> {
    let limit = q.limit.unwrap_or(20).min(100) as i64;

    let rows = sqlx::query(
        "SELECT server_name, description, icon_url, public_room_count, total_users \
         FROM directory_servers \
         ORDER BY server_name ASC \
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&state.db.pg)
    .await;

    let mut servers: Vec<ServerEntry> = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(|r| ServerEntry {
                server_name:      r.try_get("server_name").unwrap_or_default(),
                description:      r.try_get("description").ok().flatten(),
                icon_url:         r.try_get("icon_url").ok().flatten(),
                public_room_count:r.try_get::<i32, _>("public_room_count").unwrap_or(0) as u64,
                total_users:      r.try_get::<i32, _>("total_users").unwrap_or(0) as u64,
            })
            .collect(),
        Err(e) => {
            warn!("Failed to list directory servers: {}", e);
            vec![]
        }
    };

    // Always include this server first (even if not yet in directory_servers).
    let this_name = state.server_name.clone();
    if !servers.iter().any(|s| s.server_name == this_name) {
        servers.insert(
            0,
            ServerEntry {
                server_name: this_name,
                description:      Some("This Nexus server".into()),
                icon_url:         None,
                public_room_count: 0,
                total_users:       0,
            },
        );
    }

    let total_count = servers.len() as u64;
    Json(PaginatedServers { servers, total_count, next_batch: None })
}

/// `GET /api/v1/directory/rooms`
///
/// Return all publicly joinable federated rooms — from this server and any
/// remote servers in the directory.
async fn list_rooms(
    State(state): State<Arc<AppState>>,
    Query(q): Query<PaginationQuery>,
) -> Json<PaginatedRooms> {
    let limit = q.limit.unwrap_or(20).min(100) as i64;

    let rows = sqlx::query(
        "SELECT room_id, name, topic, member_count, origin_server, join_rule \
         FROM federated_rooms \
         WHERE join_rule = 'public' \
         ORDER BY member_count DESC \
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&state.db.pg)
    .await;

    let rooms: Vec<RoomEntry> = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(|r| RoomEntry {
                room_id:      r.try_get("room_id").unwrap_or_default(),
                name:         r.try_get("name").ok().flatten(),
                topic:        r.try_get("topic").ok().flatten(),
                member_count: r.try_get::<i32, _>("member_count").unwrap_or(0) as u64,
                server_name:  r.try_get("origin_server").unwrap_or_default(),
                join_rule:    r.try_get("join_rule").unwrap_or_else(|_| "public".into()),
                tags:         vec![],
            })
            .collect(),
        Err(e) => {
            warn!("Failed to list federated rooms: {}", e);
            vec![]
        }
    };

    let total_count = rooms.len() as u64;
    Json(PaginatedRooms { rooms, total_count, next_batch: None })
}

/// `GET /api/v1/directory/rooms/search?q=<query>&server=<server>&limit=<n>`
///
/// Full-text search across public room names and topics, optionally scoped
/// to a specific server.
async fn search_rooms(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SearchQuery>,
) -> Json<PaginatedRooms> {
    let query_str = format!("%{}%", q.q.unwrap_or_default());
    let server_filter = q.server;
    let limit = q.limit.unwrap_or(20).min(100) as i64;

    let rows = if let Some(ref server) = server_filter {
        sqlx::query(
            "SELECT room_id, name, topic, member_count, origin_server, join_rule \
             FROM federated_rooms \
             WHERE join_rule = 'public' \
               AND origin_server = $1 \
               AND (name ILIKE $2 OR topic ILIKE $2) \
             ORDER BY member_count DESC \
             LIMIT $3",
        )
        .bind(server)
        .bind(&query_str)
        .bind(limit)
        .fetch_all(&state.db.pg)
        .await
    } else {
        sqlx::query(
            "SELECT room_id, name, topic, member_count, origin_server, join_rule \
             FROM federated_rooms \
             WHERE join_rule = 'public' \
               AND (name ILIKE $1 OR topic ILIKE $1) \
             ORDER BY member_count DESC \
             LIMIT $2",
        )
        .bind(&query_str)
        .bind(limit)
        .fetch_all(&state.db.pg)
        .await
    };

    let rooms: Vec<RoomEntry> = match rows {
        Ok(rows) => rows
            .into_iter()
            .map(|r| RoomEntry {
                room_id:      r.try_get("room_id").unwrap_or_default(),
                name:         r.try_get("name").ok().flatten(),
                topic:        r.try_get("topic").ok().flatten(),
                member_count: r.try_get::<i32, _>("member_count").unwrap_or(0) as u64,
                server_name:  r.try_get("origin_server").unwrap_or_default(),
                join_rule:    r.try_get("join_rule").unwrap_or_else(|_| "public".into()),
                tags:         vec![],
            })
            .collect(),
        Err(e) => {
            warn!("Failed to search federated rooms: {}", e);
            vec![]
        }
    };

    let total_count = rooms.len() as u64;
    Json(PaginatedRooms { rooms, total_count, next_batch: None })
}

/// `GET /api/v1/directory/resolve/:server_name`
///
/// Return the resolved federation base URL and key information for a server.
/// Useful for clients that want to verify a server before joining a room.
async fn resolve_server(
    State(state): State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Json<Value> {
    // Check the local `federated_servers` cache first.
    let row = sqlx::query(
        "SELECT server_name, base_url, server_version, is_blocked \
         FROM federated_servers \
         WHERE server_name = $1",
    )
    .bind(&server_name)
    .fetch_optional(&state.db.pg)
    .await
    .ok()
    .flatten();

    if let Some(r) = row {
        let is_blocked: bool = r.try_get("is_blocked").unwrap_or(false);
        let base_url: Option<String> = r.try_get("base_url").ok().flatten();
        let version: Option<String> = r.try_get("server_version").ok().flatten();

        Json(json!({
            "server_name": server_name,
            "base_url": base_url.unwrap_or_else(|| format!("https://{}:8448", server_name)),
            "version": version,
            "is_blocked": is_blocked,
            "status": if is_blocked { "blocked" } else { "known" },
        }))
    } else {
        // Not in cache — return a best-effort default for the caller to verify.
        Json(json!({
            "server_name": server_name,
            "base_url": format!("https://{}:8448", server_name),
            "status": "unknown",
        }))
    }
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
