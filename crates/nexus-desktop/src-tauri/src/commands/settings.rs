//! Settings commands — server URL, user preferences, persisted via tauri-plugin-store.

use serde_json::json;
use tauri::State;
use crate::state::AppState;
use super::api_client;

/// Get all current settings as a JSON object.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let session = state.session.lock().unwrap();
    Ok(serde_json::json!({
        "server_url": session.server_url,
        "username": session.username,
        "logged_in": session.access_token.is_some(),
    }))
}

/// Set a single setting key/value.
#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    match key.as_str() {
        "server_url" => {
            if let Some(url) = value.as_str() {
                state.session.lock().unwrap().server_url = url.to_owned();
            }
        }
        _ => {
            // Unknown settings keys are silently ignored;
            // persistence is handled by the frontend via tauri-plugin-store directly.
        }
    }
    Ok(())
}

/// Convenience: set just the server URL.
#[tauri::command]
pub async fn set_server_url(
    state: State<'_, AppState>,
    url: String,
) -> Result<(), String> {
    let url = url.trim_end_matches('/').to_owned();
    state.session.lock().unwrap().server_url = url;
    Ok(())
}

/// Update the current user's display name and/or avatar URL.
///
/// Maps to `PATCH /api/v1/users/@me`. Unset fields are left unchanged on
/// the server (COALESCE semantics).
#[tauri::command]
pub async fn update_profile(
    state: State<'_, AppState>,
    display_name: Option<String>,
    avatar_url: Option<String>,
) -> Result<serde_json::Value, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let body = json!({
        "display_name": display_name,
        "avatar": avatar_url,
    });
    let resp = client
        .patch(format!("{base}/api/v1/users/@me"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        resp.json().await.map_err(|e| e.to_string())
    } else {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        Err(format!("HTTP {status}: {text}"))
    }
}
