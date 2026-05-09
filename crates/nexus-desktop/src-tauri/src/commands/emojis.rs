//! Emoji management commands — list, upload, rename, and delete server emoji.

use reqwest::multipart;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use super::api_client;
use crate::state::AppState;

/// Emoji as returned to the TypeScript frontend (camelCase).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmojiClient {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub url: Option<String>,
    pub animated: bool,
    pub managed: bool,
    pub available: bool,
}

/// List all custom emoji for a server.
#[tauri::command]
pub async fn list_emoji(
    state: State<'_, AppState>,
    server_id: Uuid,
) -> Result<Vec<EmojiClient>, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/servers/{server_id}/emojis"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    let list: Vec<EmojiClient> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(list)
}

/// Upload a new custom emoji.
///
/// `file_bytes` is the raw image data (passed from the frontend as `number[]`).
/// `content_type` is the MIME type (e.g. `"image/png"`, `"image/gif"`).
#[tauri::command]
pub async fn upload_emoji(
    state: State<'_, AppState>,
    server_id: Uuid,
    name: String,
    file_bytes: Vec<u8>,
    content_type: String,
) -> Result<EmojiClient, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;

    let image_part = multipart::Part::bytes(file_bytes)
        .mime_str(&content_type)
        .map_err(|e| e.to_string())?
        .file_name("emoji");

    let form = multipart::Form::new()
        .text("name", name)
        .part("image", image_part);

    let resp = client
        .post(format!("{base}/api/v1/servers/{server_id}/emojis"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    let emoji: EmojiClient = resp.json().await.map_err(|e| e.to_string())?;
    Ok(emoji)
}

/// Rename an existing custom emoji.
#[tauri::command]
pub async fn rename_emoji(
    state: State<'_, AppState>,
    server_id: Uuid,
    emoji_id: Uuid,
    name: String,
) -> Result<EmojiClient, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let body = serde_json::json!({ "name": name });
    let resp = client
        .patch(format!(
            "{base}/api/v1/servers/{server_id}/emojis/{emoji_id}"
        ))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    let emoji: EmojiClient = resp.json().await.map_err(|e| e.to_string())?;
    Ok(emoji)
}

/// Delete a custom emoji.
#[tauri::command]
pub async fn delete_emoji(
    state: State<'_, AppState>,
    server_id: Uuid,
    emoji_id: Uuid,
) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .delete(format!(
            "{base}/api/v1/servers/{server_id}/emojis/{emoji_id}"
        ))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    Ok(())
}
