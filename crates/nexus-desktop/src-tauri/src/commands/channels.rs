//! Channel commands.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;
use super::api_client;

/// Raw channel shape as returned by the server (snake_case).
#[derive(Deserialize, Debug)]
struct RawChannel {
    pub id: Uuid,
    pub server_id: Option<Uuid>,
    pub channel_type: String,
    pub name: Option<String>,
    pub encrypted: bool,
}

/// Channel shape expected by the frontend (camelCase, via Tauri serde).
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChannelClient {
    pub id: Uuid,
    pub server_id: Option<Uuid>,
    pub name: String,
    /// Maps from channel_type: "text"/"voice"/"announcement"
    pub kind: String,
    pub is_e2ee: bool,
}

impl From<RawChannel> for ChannelClient {
    fn from(r: RawChannel) -> Self {
        ChannelClient {
            id: r.id,
            server_id: r.server_id,
            name: r.name.unwrap_or_default(),
            kind: r.channel_type,
            is_e2ee: r.encrypted,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateChannelRequest {
    pub name: String,
    pub channel_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
}

#[tauri::command]
pub async fn create_channel(
    state: State<'_, AppState>,
    server_id: Uuid,
    name: String,
    channel_type: String,
) -> Result<ChannelClient, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{base}/api/v1/servers/{server_id}/channels"))
        .json(&CreateChannelRequest { name, channel_type, topic: None })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Server error {status}: {body}"));
    }
    let raw: RawChannel = resp.json().await.map_err(|e| e.to_string())?;
    Ok(ChannelClient::from(raw))
}

#[tauri::command]
pub async fn list_channels(
    state: State<'_, AppState>,
    server_id: Uuid,
) -> Result<Vec<ChannelClient>, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/servers/{server_id}/channels"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let raw: Vec<RawChannel> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(raw.into_iter().map(ChannelClient::from).collect())
}

#[tauri::command]
pub async fn get_channel(
    state: State<'_, AppState>,
    channel_id: Uuid,
) -> Result<ChannelClient, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/channels/{channel_id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let raw: RawChannel = resp.json().await.map_err(|e| e.to_string())?;
    Ok(ChannelClient::from(raw))
}
