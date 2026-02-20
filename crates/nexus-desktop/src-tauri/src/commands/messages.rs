//! Message commands — send and fetch plaintext messages.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;
use super::api_client;

/// Raw message shape returned by the server (snake_case JSON).
#[derive(Deserialize, Debug)]
struct RawMessage {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub author_username: Option<String>,
    pub content: String,
    pub created_at: String,
    pub edited_at: Option<String>,
    pub attachments: Option<serde_json::Value>,
}

/// Message shape expected by the TypeScript frontend (camelCase).
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageClient {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub content: String,
    pub created_at: String,
    pub edited_at: Option<String>,
}

impl From<RawMessage> for MessageClient {
    fn from(r: RawMessage) -> Self {
        MessageClient {
            id: r.id,
            channel_id: r.channel_id,
            author_id: r.author_id,
            author_username: r.author_username.unwrap_or_else(|| "Unknown".to_string()),
            content: r.content,
            created_at: r.created_at,
            edited_at: r.edited_at,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct SendMessageRequest {
    pub content: String,
    pub nonce: Option<String>,
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    channel_id: Uuid,
    content: String,
) -> Result<MessageClient, String> {
    let session = state.session_snapshot();
    if session.server_url.is_empty() {
        return Err("No server URL configured. Please log in again.".into());
    }
    if session.access_token.is_none() {
        return Err("Not authenticated. Please log in again.".into());
    }
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{base}/api/v1/channels/{channel_id}/messages"))
        .json(&SendMessageRequest { content, nonce: None })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Server error {status}: {body}"));
    }
    let raw: RawMessage = resp.json().await.map_err(|e| e.to_string())?;
    Ok(MessageClient::from(raw))
}

#[tauri::command]
pub async fn fetch_history(
    state: State<'_, AppState>,
    channel_id: Uuid,
    before: Option<Uuid>,
    limit: Option<u32>,
) -> Result<Vec<MessageClient>, String> {
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
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Server error {status}: {body}"));
    }
    let raw: Vec<RawMessage> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(raw.into_iter().map(MessageClient::from).collect())
}
