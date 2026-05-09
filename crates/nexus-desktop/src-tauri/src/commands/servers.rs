//! Server commands — list, retrieve, and create servers.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use super::{api_client, friendly_api_error, friendly_network_error};
use crate::state::AppState;

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
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &body));
    }
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
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &body));
    }
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
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &text));
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
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &text));
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
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &text));
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
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &text));
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
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &text));
    }
    Ok(())
}

// ─── Server management commands ─────────────────────────────────────────────

/// Update server settings (name, description, public flag, etc.).
#[tauri::command]
pub async fn update_server(
    state: State<'_, AppState>,
    server_id: Uuid,
    name: Option<String>,
    description: Option<String>,
    is_public: Option<bool>,
    region: Option<String>,
    require_2fa: Option<bool>,
    spam_window_secs: Option<i32>,
    spam_max_messages: Option<i32>,
) -> Result<ServerClient, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let body = serde_json::json!({
        "name": name,
        "description": description,
        "is_public": is_public,
        "region": region,
        "require_2fa": require_2fa,
        "spam_window_secs": spam_window_secs,
        "spam_max_messages": spam_max_messages,
    });
    let resp = client
        .patch(format!("{base}/api/v1/servers/{server_id}"))
        .json(&body)
        .send()
        .await
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &text));
    }
    let raw: RawServer = resp.json().await.map_err(|e| e.to_string())?;
    Ok(ServerClient::from(raw))
}

/// Delete a server permanently.
#[tauri::command]
pub async fn delete_server(state: State<'_, AppState>, server_id: Uuid) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .delete(format!("{base}/api/v1/servers/{server_id}"))
        .send()
        .await
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &text));
    }
    Ok(())
}

/// Transfer server ownership to another member.
#[tauri::command]
pub async fn transfer_server_ownership(
    state: State<'_, AppState>,
    server_id: Uuid,
    new_owner_id: String,
) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let body = serde_json::json!({ "new_owner_id": new_owner_id });
    let resp = client
        .post(format!(
            "{base}/api/v1/servers/{server_id}/transfer-ownership"
        ))
        .json(&body)
        .send()
        .await
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &text));
    }
    Ok(())
}

/// Leave a server (non-owner members only).
#[tauri::command]
pub async fn leave_server(state: State<'_, AppState>, server_id: Uuid) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{base}/api/v1/servers/{server_id}/leave"))
        .send()
        .await
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &text));
    }
    Ok(())
}

// ─── Invite commands ─────────────────────────────────────────────────────────

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InviteClient {
    pub code: String,
    pub server_id: String,
    pub max_uses: Option<i64>,
    pub uses: i64,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Deserialize, Debug, Clone)]
struct RawInvite {
    pub code: String,
    pub server_id: serde_json::Value,
    pub max_uses: Option<i64>,
    pub uses: i64,
    pub expires_at: Option<String>,
    pub created_at: String,
}

impl From<RawInvite> for InviteClient {
    fn from(r: RawInvite) -> Self {
        Self {
            code: r.code,
            server_id: r.server_id.to_string().trim_matches('"').to_string(),
            max_uses: r.max_uses,
            uses: r.uses,
            expires_at: r.expires_at,
            created_at: r.created_at,
        }
    }
}

/// List all active invites for a server.
#[tauri::command]
pub async fn list_server_invites(
    state: State<'_, AppState>,
    server_id: Uuid,
) -> Result<Vec<InviteClient>, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/servers/{server_id}/invites"))
        .send()
        .await
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &text));
    }
    let raw: Vec<RawInvite> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(raw.into_iter().map(InviteClient::from).collect())
}

/// The API only returns `{ "code": "..." }` for invite creation.
#[derive(Deserialize)]
struct CreateInviteResponse {
    code: String,
}

/// Create a new invite for a server.
#[tauri::command]
pub async fn create_invite(
    state: State<'_, AppState>,
    server_id: Uuid,
    max_uses: Option<i64>,
    max_age_secs: Option<i64>,
) -> Result<InviteClient, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let body = serde_json::json!({
        "max_uses": max_uses,
        "max_age_secs": max_age_secs,
    });
    let resp = client
        .post(format!("{base}/api/v1/servers/{server_id}/invites"))
        .json(&body)
        .send()
        .await
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &text));
    }
    let r: CreateInviteResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(InviteClient {
        code: r.code,
        server_id: server_id.to_string(),
        max_uses,
        uses: 0,
        expires_at: None,
        created_at: String::new(),
    })
}

/// Delete (revoke) a server invite by code.
#[tauri::command]
pub async fn delete_invite(
    state: State<'_, AppState>,
    server_id: Uuid,
    code: String,
) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .delete(format!("{base}/api/v1/servers/{server_id}/invites/{code}"))
        .send()
        .await
        .map_err(friendly_network_error)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &text));
    }
    Ok(())
}
