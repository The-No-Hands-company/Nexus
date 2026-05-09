//! Friends, relationships, DMs, user search and user profile commands.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::commands::api_client;
use crate::state::AppState;

// ── Shared types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct UserBrief {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct RelationshipEntry {
    pub id: String,
    pub direction: String,
    pub status: String,
    pub user: UserBrief,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct DmChannel {
    pub id: String,
    pub channel_type: String,
    pub name: Option<String>,
    pub last_message_id: Option<String>,
    pub recipients: Vec<UserBrief>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn check_success(status: reqwest::StatusCode, body: &str) -> Result<(), String> {
    if !status.is_success() {
        return Err(body.to_owned());
    }
    Ok(())
}

async fn get_text(resp: reqwest::Response) -> (reqwest::StatusCode, String) {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    (status, body)
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// List all relationships (friends, pending, blocked) for the current user.
#[tauri::command]
pub async fn list_relationships(
    state: State<'_, AppState>,
) -> Result<Vec<RelationshipEntry>, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/users/@me/relationships"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json::<Vec<RelationshipEntry>>()
        .await
        .map_err(|e| e.to_string())
}

/// Send a friend request to a user by their username.
#[tauri::command]
pub async fn send_friend_request(
    state: State<'_, AppState>,
    username: String,
) -> Result<RelationshipEntry, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{base}/api/v1/users/@me/relationships"))
        .json(&serde_json::json!({ "username": username }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json::<RelationshipEntry>()
        .await
        .map_err(|e| e.to_string())
}

/// Accept, deny, or block a relationship (action: "accept" | "deny" | "block").
#[tauri::command]
pub async fn update_relationship(
    state: State<'_, AppState>,
    user_id: Uuid,
    action: String,
) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .patch(format!("{base}/api/v1/users/@me/relationships/{user_id}"))
        .json(&serde_json::json!({ "action": action }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let (status, body) = get_text(resp).await;
    check_success(status, &body)
}

/// Remove a friend, cancel an outgoing request, or unblock a user.
#[tauri::command]
pub async fn delete_relationship(state: State<'_, AppState>, user_id: Uuid) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .delete(format!("{base}/api/v1/users/@me/relationships/{user_id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let (status, body) = get_text(resp).await;
    check_success(status, &body)
}

/// Search users by username prefix.
#[tauri::command]
pub async fn search_users(state: State<'_, AppState>, q: String) -> Result<Vec<UserBrief>, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/users/search"))
        .query(&[("q", &q)])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json::<Vec<UserBrief>>()
        .await
        .map_err(|e| e.to_string())
}

/// List all DM channels for the current user.
#[tauri::command]
pub async fn list_dm_channels(state: State<'_, AppState>) -> Result<Vec<DmChannel>, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/users/@me/channels"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json::<Vec<DmChannel>>()
        .await
        .map_err(|e| e.to_string())
}

/// Open or create a DM channel with a user.
#[tauri::command]
pub async fn create_dm(
    state: State<'_, AppState>,
    recipient_id: Uuid,
) -> Result<DmChannel, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{base}/api/v1/users/@me/channels"))
        .json(&serde_json::json!({ "recipient_id": recipient_id }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json::<DmChannel>().await.map_err(|e| e.to_string())
}

/// List members of a server.
#[tauri::command]
pub async fn list_members(
    state: State<'_, AppState>,
    server_id: Uuid,
) -> Result<Vec<serde_json::Value>, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/servers/{server_id}/members"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json::<Vec<serde_json::Value>>()
        .await
        .map_err(|e| e.to_string())
}

/// Fetch the enriched profile of a user (mutual servers, mutual friends, etc.)
#[tauri::command]
pub async fn get_user_profile(
    state: State<'_, AppState>,
    user_id: Uuid,
) -> Result<serde_json::Value, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/users/{user_id}/profile"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())
}
