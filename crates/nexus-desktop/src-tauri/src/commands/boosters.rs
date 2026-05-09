//! Tauri commands for server boost / supporter tier system.
//!
//! Covers: listing boosters, adding/removing boost slots, reading tier info,
//! and setting the server vanity URL (tier 2+).

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use super::api_client;
use crate::state::AppState;

// ─── Client-side types ────────────────────────────────────────────────────────

/// Server boost tier info — mirrors the API's `BoostTierInfo`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct BoostTierInfo {
    pub tier: i16,
    pub booster_count: i64,
    pub extra_emoji_slots: i32,
    pub upload_limit_bytes: i64,
    pub vanity_url_available: bool,
    pub current_vanity_code: Option<String>,
}

/// A single active boost slot — mirrors the API's `BoosterEntry`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct BoosterEntry {
    pub id: String,
    pub user_id: String,
    pub server_id: String,
    pub slot: i16,
    pub started_at: String,
    pub expires_at: Option<String>,
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// GET /api/v1/servers/{server_id}/boost-tier — current tier + perks.
#[tauri::command]
pub async fn get_server_boost_tier(
    state: State<'_, AppState>,
    server_id: Uuid,
) -> Result<BoostTierInfo, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/servers/{server_id}/boost-tier"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_default());
    }
    resp.json::<BoostTierInfo>()
        .await
        .map_err(|e| e.to_string())
}

/// GET /api/v1/servers/{server_id}/boosters — list active boosters.
#[tauri::command]
pub async fn list_server_boosters(
    state: State<'_, AppState>,
    server_id: Uuid,
) -> Result<Vec<BoosterEntry>, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/servers/{server_id}/boosters"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_default());
    }
    resp.json::<Vec<BoosterEntry>>()
        .await
        .map_err(|e| e.to_string())
}

/// POST /api/v1/servers/{server_id}/boost — add a boost slot.
#[tauri::command]
pub async fn boost_server(
    state: State<'_, AppState>,
    server_id: Uuid,
) -> Result<BoosterEntry, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{base}/api/v1/servers/{server_id}/boost"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_default());
    }
    resp.json::<BoosterEntry>().await.map_err(|e| e.to_string())
}

/// DELETE /api/v1/servers/{server_id}/boost/{slot} — remove a boost slot.
#[tauri::command]
pub async fn remove_boost(
    state: State<'_, AppState>,
    server_id: Uuid,
    slot: i16,
) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .delete(format!("{base}/api/v1/servers/{server_id}/boost/{slot}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_default());
    }
    Ok(())
}

/// PATCH /api/v1/servers/{server_id}/vanity-url — set vanity invite code (tier 2+).
#[tauri::command]
pub async fn set_vanity_url(
    state: State<'_, AppState>,
    server_id: Uuid,
    vanity_code: String,
) -> Result<(), String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let body = serde_json::json!({ "code": vanity_code });
    let resp = client
        .patch(format!("{base}/api/v1/servers/{server_id}/vanity-url"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_default());
    }
    Ok(())
}
