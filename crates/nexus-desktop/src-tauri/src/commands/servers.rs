//! Server commands — list, retrieve, and create servers.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;
use super::api_client;

/// Raw shape returned by the Nexus API (snake_case)
#[derive(Deserialize, Debug, Clone)]
struct RawServer {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub member_count: Option<serde_json::Value>,
    pub owner_id: Uuid,
}

/// Typed shape returned to the TypeScript frontend (camelCase)
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServerClient {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub member_count: Option<i64>,
    pub owner_id: Uuid,
}

impl From<RawServer> for ServerClient {
    fn from(r: RawServer) -> Self {
        let member_count = match r.member_count {
            Some(serde_json::Value::Number(n)) => n.as_i64(),
            _ => None,
        };
        Self {
            id: r.id,
            name: r.name,
            icon: r.icon,
            member_count,
            owner_id: r.owner_id,
        }
    }
}

#[tauri::command]
pub async fn list_servers(state: State<'_, AppState>) -> Result<Vec<ServerClient>, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/servers"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let raw: Vec<RawServer> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(raw.into_iter().map(ServerClient::from).collect())
}

#[tauri::command]
pub async fn get_server(
    state: State<'_, AppState>,
    server_id: Uuid,
) -> Result<ServerClient, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/servers/{server_id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let raw: RawServer = resp.json().await.map_err(|e| e.to_string())?;
    Ok(ServerClient::from(raw))
}

#[derive(Deserialize)]
pub struct CreateServerPayload {
    pub name: String,
    pub is_public: Option<bool>,
}

#[tauri::command]
pub async fn create_server(
    state: State<'_, AppState>,
    name: String,
    is_public: Option<bool>,
) -> Result<ServerClient, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let body = serde_json::json!({ "name": name, "is_public": is_public.unwrap_or(false) });
    let resp = client
        .post(format!("{base}/api/v1/servers"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    let raw: RawServer = resp.json().await.map_err(|e| e.to_string())?;
    Ok(ServerClient::from(raw))
}

// ─── Role commands ──────────────────────────────────────────────────────────

/// Raw role shape from the API
#[derive(Deserialize, Debug, Clone)]
struct RawRole {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub color: Option<i32>,
    pub hoist: bool,
    pub icon: Option<String>,
    pub position: i32,
    pub permissions: i64,
    pub mentionable: bool,
    pub is_default: bool,
}

/// Role shape returned to the TypeScript frontend (camelCase)
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoleClient {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub color: Option<i32>,
    pub hoist: bool,
    pub icon: Option<String>,
    pub position: i32,
    pub permissions: i64,
    pub mentionable: bool,
    pub is_default: bool,
}

impl From<RawRole> for RoleClient {
    fn from(r: RawRole) -> Self {
        Self {
            id: r.id,
            server_id: r.server_id,
            name: r.name,
            color: r.color,
            hoist: r.hoist,
            icon: r.icon,
            position: r.position,
            permissions: r.permissions,
            mentionable: r.mentionable,
            is_default: r.is_default,
        }
    }
}

/// List all roles for a server.
#[tauri::command]
pub async fn list_roles(
    state: State<'_, AppState>,
    server_id: Uuid,
) -> Result<Vec<RoleClient>, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/servers/{server_id}/roles"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    let raw: Vec<RawRole> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(raw.into_iter().map(RoleClient::from).collect())
}

/// Create a new role in a server.
#[tauri::command]
pub async fn create_role(
    state: State<'_, AppState>,
    server_id: Uuid,
    name: String,
    color: Option<i32>,
    permissions: Option<i64>,
    hoist: Option<bool>,
    mentionable: Option<bool>,
) -> Result<RoleClient, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let body = serde_json::json!({
        "name": name,
        "color": color,
        "permissions": permissions,
        "hoist": hoist,
        "mentionable": mentionable,
    });
    let resp = client
        .post(format!("{base}/api/v1/servers/{server_id}/roles"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    let raw: RawRole = resp.json().await.map_err(|e| e.to_string())?;
    Ok(RoleClient::from(raw))
}

/// Update an existing role.
#[tauri::command]
pub async fn update_role(
    state: State<'_, AppState>,
    server_id: Uuid,
    role_id: Uuid,
    name: Option<String>,
    color: Option<i32>,
    permissions: Option<i64>,
    hoist: Option<bool>,
    mentionable: Option<bool>,
) -> Result<RoleClient, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let body = serde_json::json!({
        "name": name,
        "color": color,
        "permissions": permissions,
        "hoist": hoist,
        "mentionable": mentionable,
    });
    let resp = client
        .patch(format!("{base}/api/v1/servers/{server_id}/roles/{role_id}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    let raw: RawRole = resp.json().await.map_err(|e| e.to_string())?;
    Ok(RoleClient::from(raw))
}

/// Delete a role from a server.
#[tauri::command]
pub async fn delete_role(
    state: State<'_, AppState>,
    server_id: Uuid,
    role_id: Uuid,
) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .delete(format!("{base}/api/v1/servers/{server_id}/roles/{role_id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    Ok(())
}
