//! E2EE commands — device registration, key bundles, sending encrypted messages.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;
use super::api_client;

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterDeviceDto {
    pub name: String,
    pub device_type: Option<String>,
    pub identity_key: String,
    pub signed_pre_key: String,
    pub signed_pre_key_sig: String,
    pub signed_pre_key_id: i32,
    pub one_time_pre_keys: Vec<OtpkDto>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OtpkDto {
    pub key_id: i32,
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendEncryptedDto {
    pub ciphertext_map: serde_json::Value,
    pub attachment_meta: Option<serde_json::Value>,
    pub client_ts: Option<String>,
}

/// Register a new device and upload initial key material.
#[tauri::command]
pub async fn register_device(
    state: State<'_, AppState>,
    device: RegisterDeviceDto,
) -> Result<serde_json::Value, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{base}/api/v1/devices"))
        .json(&device)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

/// Fetch key bundles for all devices of a user (X3DH initiator step).
#[tauri::command]
pub async fn get_key_bundle(
    state: State<'_, AppState>,
    user_id: Uuid,
) -> Result<serde_json::Value, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/users/{user_id}/key-bundle"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

/// Send an encrypted message to a channel.
#[tauri::command]
pub async fn send_encrypted_message(
    state: State<'_, AppState>,
    channel_id: Uuid,
    ciphertext_map: serde_json::Value,
    attachment_meta: Option<serde_json::Value>,
    client_ts: Option<String>,
) -> Result<serde_json::Value, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{base}/api/v1/channels/{channel_id}/encrypted-messages"))
        .json(&SendEncryptedDto {
            ciphertext_map,
            attachment_meta,
            client_ts,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

/// Fetch encrypted message history for a channel.
#[tauri::command]
pub async fn fetch_encrypted_history(
    state: State<'_, AppState>,
    channel_id: Uuid,
    before_sequence: Option<i64>,
    limit: Option<i64>,
) -> Result<serde_json::Value, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;
    let mut url = format!(
        "{base}/api/v1/channels/{channel_id}/encrypted-messages?limit={}",
        limit.unwrap_or(50).min(100)
    );
    if let Some(seq) = before_sequence {
        url.push_str(&format!("&before_sequence={seq}"));
    }
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}
