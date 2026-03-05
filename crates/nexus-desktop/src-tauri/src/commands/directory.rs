//! Server directory & federated room browser commands.
//!
//! Maps to the `/api/v1/directory/*` endpoints added in v0.8:
//!   GET  /api/v1/directory/servers          — list known public servers
//!   GET  /api/v1/directory/rooms            — list public federated rooms
//!   GET  /api/v1/directory/rooms/search     — search rooms by name/topic
//!   POST /api/v1/directory/rooms/join       — federated room join

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;
use super::api_client;

// ─── Response types (camelCase for TypeScript) ─────────────────────────────

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryServer {
    pub server_name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub public_room_count: u64,
    pub total_users: u64,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryRoom {
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub member_count: u64,
    pub server_name: String,
    pub join_rule: String,
    pub tags: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryServerList {
    pub servers: Vec<DirectoryServer>,
    pub total_count: u64,
    pub next_batch: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryRoomList {
    pub rooms: Vec<DirectoryRoom>,
    pub total_count: u64,
    pub next_batch: Option<String>,
}

// ─── Raw API shapes (snake_case) ────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct RawServer {
    server_name: String,
    description: Option<String>,
    icon_url: Option<String>,
    public_room_count: Option<serde_json::Value>,
    total_users: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
struct RawRoom {
    room_id: String,
    name: Option<String>,
    topic: Option<String>,
    member_count: Option<serde_json::Value>,
    server_name: String,
    join_rule: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct RawServerList {
    servers: Vec<RawServer>,
    total_count: Option<serde_json::Value>,
    next_batch: Option<String>,
}

#[derive(Deserialize, Debug)]
struct RawRoomList {
    rooms: Vec<RawRoom>,
    total_count: Option<serde_json::Value>,
    next_batch: Option<String>,
}

fn parse_u64(v: &Option<serde_json::Value>) -> u64 {
    match v {
        Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0),
        _ => 0,
    }
}

impl From<RawServer> for DirectoryServer {
    fn from(r: RawServer) -> Self {
        Self {
            server_name: r.server_name,
            description: r.description,
            icon_url: r.icon_url,
            public_room_count: parse_u64(&r.public_room_count),
            total_users: parse_u64(&r.total_users),
        }
    }
}

impl From<RawRoom> for DirectoryRoom {
    fn from(r: RawRoom) -> Self {
        Self {
            room_id: r.room_id,
            name: r.name,
            topic: r.topic,
            member_count: parse_u64(&r.member_count),
            server_name: r.server_name,
            join_rule: r.join_rule.unwrap_or_else(|| "public".into()),
            tags: r.tags.unwrap_or_default(),
        }
    }
}

// ─── Commands ───────────────────────────────────────────────────────────────

/// List all servers known to the local directory (including this one).
#[tauri::command]
pub async fn directory_list_servers(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<DirectoryServerList, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(50);
    let resp = client
        .get(format!("{base}/api/v1/directory/servers"))
        .query(&[("limit", limit.to_string())])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    let raw: RawServerList = resp.json().await.map_err(|e| e.to_string())?;
    let total_count = parse_u64(&raw.total_count);
    Ok(DirectoryServerList {
        servers: raw.servers.into_iter().map(DirectoryServer::from).collect(),
        total_count,
        next_batch: raw.next_batch,
    })
}

/// List all public rooms across federated servers.
#[tauri::command]
pub async fn directory_list_rooms(
    state: State<'_, AppState>,
    server: Option<String>,
    limit: Option<u32>,
) -> Result<DirectoryRoomList, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(50);
    let mut params: Vec<(&str, String)> = vec![("limit", limit.to_string())];
    if let Some(ref s) = server {
        params.push(("server", s.clone()));
    }
    let resp = client
        .get(format!("{base}/api/v1/directory/rooms"))
        .query(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    let raw: RawRoomList = resp.json().await.map_err(|e| e.to_string())?;
    let total_count = parse_u64(&raw.total_count);
    Ok(DirectoryRoomList {
        rooms: raw.rooms.into_iter().map(DirectoryRoom::from).collect(),
        total_count,
        next_batch: raw.next_batch,
    })
}

/// Search public rooms by name/topic, optionally filtered to one server.
#[tauri::command]
pub async fn directory_search_rooms(
    state: State<'_, AppState>,
    query: String,
    server: Option<String>,
    limit: Option<u32>,
) -> Result<DirectoryRoomList, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(50);
    let mut params: Vec<(&str, String)> = vec![
        ("q", query),
        ("limit", limit.to_string()),
    ];
    if let Some(ref s) = server {
        params.push(("server", s.clone()));
    }
    let resp = client
        .get(format!("{base}/api/v1/directory/rooms/search"))
        .query(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    let raw: RawRoomList = resp.json().await.map_err(|e| e.to_string())?;
    let total_count = parse_u64(&raw.total_count);
    Ok(DirectoryRoomList {
        rooms: raw.rooms.into_iter().map(DirectoryRoom::from).collect(),
        total_count,
        next_batch: raw.next_batch,
    })
}

/// Initiate a federated join for a room via the make_join → send_join protocol.
#[tauri::command]
pub async fn directory_join_room(
    state: State<'_, AppState>,
    room_id: String,
) -> Result<serde_json::Value, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{base}/api/v1/directory/rooms/join"))
        .json(&serde_json::json!({ "room_id": room_id }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json::<serde_json::Value>().await.map_err(|e| e.to_string())
}
