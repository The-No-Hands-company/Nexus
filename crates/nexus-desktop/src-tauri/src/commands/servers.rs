//! Server commands — list and retrieve servers.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;
use super::api_client;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerSummary {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub member_count: Option<i64>,
    pub owner_id: Uuid,
}

#[tauri::command]
pub async fn list_servers(state: State<'_, AppState>) -> Result<Vec<ServerSummary>, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/servers"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_server(
    state: State<'_, AppState>,
    server_id: Uuid,
) -> Result<serde_json::Value, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/servers/{server_id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}
