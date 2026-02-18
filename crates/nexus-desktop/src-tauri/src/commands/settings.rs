//! Settings commands — server URL, user preferences, persisted via tauri-plugin-store.

use tauri::State;
use crate::state::AppState;

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
