//! Tauri commands for bot application management.
//!
//! Covers the server-admin perspective: list installed bots, install a new
//! bot by its application ID, uninstall a bot, and look up public bot info
//! before installing.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use super::api_client;
use crate::state::AppState;

// ─── Client-side types ────────────────────────────────────────────────────────

/// Installed bot record (mirrors server `BotServerInstall`).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct BotInstall {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub server_id: Uuid,
    pub installed_by: Uuid,
    pub scopes: Vec<String>,
    pub permissions: i64,
    pub installed_at: String,
}

/// Public bot info returned by the unauthenticated lookup endpoint.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct PublicBotInfo {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub avatar: Option<String>,
    pub verified: bool,
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// List bots currently installed in a server.
#[tauri::command]
pub async fn list_server_bots(
    state: State<'_, AppState>,
    server_id: Uuid,
) -> Result<Vec<BotInstall>, String> {
    let session = state.session_snapshot();
    let (client, base_url) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!(
            "{base_url}/api/v1/servers/{server_id}/integrations"
        ))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to list bots: {msg}"));
    }
    resp.json::<Vec<BotInstall>>()
        .await
        .map_err(|e| e.to_string())
}

/// Install a bot into a server by its application UUID.
#[tauri::command]
pub async fn install_bot(
    state: State<'_, AppState>,
    server_id: Uuid,
    bot_id: Uuid,
    permissions: Option<i64>,
) -> Result<BotInstall, String> {
    let session = state.session_snapshot();
    let (client, base_url) = api_client(&session).map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "bot_id": bot_id,
        "permissions": permissions.unwrap_or(0),
        "scopes": ["bot"],
    });

    let resp = client
        .post(format!(
            "{base_url}/api/v1/servers/{server_id}/integrations"
        ))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to install bot: {msg}"));
    }
    resp.json::<BotInstall>().await.map_err(|e| e.to_string())
}

/// Uninstall a bot from a server.
#[tauri::command]
pub async fn uninstall_bot(
    state: State<'_, AppState>,
    server_id: Uuid,
    bot_id: Uuid,
) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base_url) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .delete(format!(
            "{base_url}/api/v1/servers/{server_id}/integrations/{bot_id}"
        ))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to uninstall bot: {msg}"));
    }
    Ok(())
}

/// Look up public info for a bot application by its UUID.
/// Returns `None` if the bot does not exist or is not public.
#[tauri::command]
pub async fn get_public_bot_info(
    state: State<'_, AppState>,
    bot_id: Uuid,
) -> Result<Option<PublicBotInfo>, String> {
    let session = state.session_snapshot();
    let (client, base_url) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base_url}/api/v1/bots/{bot_id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to fetch bot info: {msg}"));
    }
    let info = resp
        .json::<PublicBotInfo>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(Some(info))
}
