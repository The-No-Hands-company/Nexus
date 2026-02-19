//! Auth commands — login, logout, token refresh, current user.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;
use super::api_client;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub presence: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// Register a new account and immediately store the resulting credentials.
#[tauri::command]
pub async fn register(
    state: State<'_, AppState>,
    username: String,
    email: String,
    password: String,
) -> Result<AuthResponse, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;

    let resp = client
        .post(format!("{base}/api/v1/auth/register"))
        .json(&RegisterRequest { username, email, password })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Registration failed ({status}): {body}"));
    }

    let auth: AuthResponse = resp.json().await.map_err(|e| e.to_string())?;

    {
        let mut session = state.session.lock().unwrap();
        session.access_token = Some(auth.access_token.clone());
        session.refresh_token = Some(auth.refresh_token.clone());
        session.user_id = Some(auth.user_id);
        session.username = Some(auth.username.clone());
    }

    Ok(auth)
}

/// Log in and store credentials in `AppState`.
#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> Result<AuthResponse, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;

    let resp = client
        .post(format!("{base}/api/v1/auth/login"))
        .json(&LoginRequest { username, password })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Login failed ({status}): {body}"));
    }

    let auth: AuthResponse = resp.json().await.map_err(|e| e.to_string())?;

    // Persist in state
    {
        let mut session = state.session.lock().unwrap();
        session.access_token = Some(auth.access_token.clone());
        session.refresh_token = Some(auth.refresh_token.clone());
        session.user_id = Some(auth.user_id);
        session.username = Some(auth.username.clone());
    }

    Ok(auth)
}

/// Clear session credentials.
#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let mut session = state.session.lock().unwrap();
    *session = crate::state::Session {
        server_url: session.server_url.clone(),
        ..Default::default()
    };
    Ok(())
}

/// Refresh the access token using the stored refresh token.
#[tauri::command]
pub async fn refresh_token(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let session = state.session_snapshot();
    let refresh = session
        .refresh_token
        .as_ref()
        .ok_or("No refresh token stored")?
        .clone();

    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;

    let resp = client
        .post(format!("{base}/api/v1/auth/refresh"))
        .json(&serde_json::json!({ "refresh_token": refresh }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err("Token refresh failed".into());
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let new_token = body["access_token"]
        .as_str()
        .ok_or("Missing access_token in response")?
        .to_owned();

    state.session.lock().unwrap().access_token = Some(new_token.clone());
    Ok(new_token)
}

/// Fetch the currently logged-in user's profile.
#[tauri::command]
pub async fn get_current_user(
    state: State<'_, AppState>,
) -> Result<CurrentUser, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;

    let resp = client
        .get(format!("{base}/api/v1/users/@me"))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Failed to fetch user: {}", resp.status()));
    }

    resp.json::<CurrentUser>().await.map_err(|e| e.to_string())
}
