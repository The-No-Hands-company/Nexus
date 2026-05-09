//! Presence commands.

use super::api_client;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn update_presence(
    state: State<'_, AppState>,
    presence: String,
    status: Option<String>,
) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    client
        .post(format!("{base}/api/v1/users/@me/presence"))
        .json(&serde_json::json!({ "presence": presence, "status": status }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
