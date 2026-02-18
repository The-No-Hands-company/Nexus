//! Message commands — send and fetch plaintext messages.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;
use super::api_client;

#[derive(Serialize, Deserialize, Debug)]
pub struct SendMessageRequest {
    pub content: String,
    pub nonce: Option<String>,
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    channel_id: Uuid,
    content: String,
) -> Result<serde_json::Value, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{base}/api/v1/channels/{channel_id}/messages"))
        .json(&SendMessageRequest { content, nonce: None })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_history(
    state: State<'_, AppState>,
    channel_id: Uuid,
    before: Option<Uuid>,
    limit: Option<u32>,
) -> Result<serde_json::Value, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let mut url = format!(
        "{base}/api/v1/channels/{channel_id}/messages?limit={}",
        limit.unwrap_or(50).min(100)
    );
    if let Some(b) = before {
        url.push_str(&format!("&before={b}"));
    }
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}
