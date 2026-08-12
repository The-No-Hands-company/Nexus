//! Auth commands — login, logout, token refresh, current user.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use super::{api_client, friendly_api_error, friendly_network_error};
use crate::state::AppState;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthUserInfo {
    pub id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthResponse {
    /// Ecosystem session token. Sent to apps as the `nexus_session` cookie.
    pub token: String,
    pub user: AuthUserInfo,
}

/// Where accounts live: the ecosystem identity service, not the app server.
///
/// Overridable so a local ecosystem can be pointed at, but the default is the
/// real one — a desktop client that guessed wrong here would send a password
/// somewhere it does not belong.
fn auth_base() -> String {
    std::env::var("NEXUS_AUTH_URL")
        .unwrap_or_else(|_| "https://auth.tnhc.dev".to_string())
        .trim_end_matches('/')
        .to_owned()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub presence: String,
}

/// Where someone with no account is sent.
///
/// Registration is invite-only and deliberately not implemented here: a
/// request has to be approved by an operator before an account exists, and
/// that flow already exists on the web.
#[tauri::command]
pub fn request_access_url() -> String {
    "https://app.tnhc.dev/request".to_string()
}

/// Log in and store credentials in `AppState`.
#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> Result<AuthResponse, String> {
    // Goes to Auth, not to the app server. There is one account for the whole
    // ecosystem and only Auth holds it.
    let client = reqwest::Client::new();
    let base = auth_base();

    let resp = client
        .post(format!("{base}/api/v1/auth/login"))
        .json(&LoginRequest { username, password })
        .send()
        .await
        .map_err(friendly_network_error)?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &body));
    }

    let auth: AuthResponse = resp.json().await.map_err(|e| e.to_string())?;

    // Persist in state
    {
        let mut session = state.session.lock().unwrap();
        session.session_token = Some(auth.token.clone());
        session.user_id = Some(auth.user.id);
        session.username = Some(auth.user.username.clone());
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

/// Fetch the currently logged-in user's profile.
#[tauri::command]
pub async fn get_current_user(state: State<'_, AppState>) -> Result<CurrentUser, String> {
    let session = state.session_snapshot();
    let (client, base) = api_client(&session).map_err(|e| e.to_string())?;

    let resp = client
        .get(format!("{base}/api/v1/users/@me"))
        .send()
        .await
        .map_err(friendly_network_error)?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(friendly_api_error(status, &body));
    }

    resp.json::<CurrentUser>().await.map_err(|e| e.to_string())
}
