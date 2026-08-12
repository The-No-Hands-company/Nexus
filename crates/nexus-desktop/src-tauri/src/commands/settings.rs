//! Settings commands — server URL, user preferences, persisted via tauri-plugin-store.

use serde_json::json;
use tauri::State;

use super::api_client;
use crate::state::AppState;

/// Get all current settings as a JSON object.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let session = state.session.lock().unwrap();
    Ok(serde_json::json!({
        "server_url": session.server_url,
        "username": session.username,
        "logged_in": session.session_token.is_some(),
    }))
}

/// Set a single setting key/value.
#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    if key.as_str() == "server_url"
        && let Some(url) = value.as_str()
    {
        state.session.lock().unwrap().server_url = url.to_owned();
    }
    Ok(())
}

/// Convenience: set just the server URL.
#[tauri::command]
pub async fn set_server_url(state: State<'_, AppState>, url: String) -> Result<(), String> {
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
        if resp.status().as_u16() == 204 {
            Ok(serde_json::json!({ "ok": true }))
        } else {
            resp.json().await.map_err(|e| e.to_string())
        }
    } else {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        Err(format!("HTTP {status}: {text}"))
    }
}

/// Change the current password.
#[tauri::command]
pub async fn change_password(
    state: State<'_, AppState>,
    current_password: String,
    new_password: String,
) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{base}/api/v1/users/@me/change-password"))
        .json(&json!({
            "current_password": current_password,
            "new_password": new_password,
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        Err(format!("HTTP {status}: {text}"))
    }
}

/// Request account deletion with a password confirmation.
#[tauri::command]
pub async fn delete_account(state: State<'_, AppState>, password: String) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .delete(format!("{base}/api/v1/users/@me"))
        .json(&json!({ "password": password }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        Err(format!("HTTP {status}: {text}"))
    }
}

/// Cancel a pending account deletion.
#[tauri::command]
pub async fn cancel_account_deletion(state: State<'_, AppState>) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{base}/api/v1/users/@me/cancel-deletion"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        Err(format!("HTTP {status}: {text}"))
    }
}
