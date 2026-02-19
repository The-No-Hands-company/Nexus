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
