//! Webhook management commands — list, create, and delete server webhooks.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;
use super::api_client;

/// Webhook type.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WebhookType {
    Incoming,
    Outgoing,
}

/// Webhook as returned to the TypeScript frontend (camelCase).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WebhookClient {
    pub id: Uuid,
    pub webhook_type: WebhookType,
    pub server_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub name: String,
    pub avatar: Option<String>,
    /// Token — only present when webhook is freshly created or listed by owner.
    pub token: Option<String>,
    /// Pre-built execution URL stored in DB (incoming webhooks).
    pub url: Option<String>,
    /// Target URL (outgoing webhooks).
    pub events: Vec<String>,
    pub active: bool,
    pub delivery_count: i64,
}

/// List all webhooks for a server.
#[tauri::command]
pub async fn list_webhooks(
    state: State<'_, AppState>,
    server_id: Uuid,
) -> Result<Vec<WebhookClient>, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/servers/{server_id}/webhooks"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    let list: Vec<WebhookClient> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(list)
}

/// Create an incoming webhook for a channel.
#[tauri::command]
pub async fn create_incoming_webhook(
    state: State<'_, AppState>,
    channel_id: Uuid,
    name: String,
) -> Result<WebhookClient, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let body = serde_json::json!({ "name": name });
    let resp = client
        .post(format!("{base}/api/v1/channels/{channel_id}/webhooks"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    let wh: WebhookClient = resp.json().await.map_err(|e| e.to_string())?;
    Ok(wh)
}

/// Delete a webhook.
#[tauri::command]
pub async fn delete_webhook(
    state: State<'_, AppState>,
    webhook_id: Uuid,
) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .delete(format!("{base}/api/v1/webhooks/{webhook_id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    Ok(())
}
