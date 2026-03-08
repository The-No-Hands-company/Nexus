//! Tauri commands — the bridge between the React frontend and the Nexus REST API.
//!
//! Each sub-module groups commands by domain, mirroring the server-side route structure.
//! All HTTP calls use the `reqwest` client with the base URL from [`AppState::session`].

pub mod auth;
pub mod boosters;
pub mod directory;
pub mod bots;
pub mod channels;
pub mod e2ee;
pub mod emojis;
pub mod friends;
pub mod messages;
pub mod presence;
pub mod servers;
pub mod settings;
pub mod voice;
pub mod webhooks;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;

use crate::state::Session;

/// Turn a reqwest send/connect error into a user-friendly message.
pub(crate) fn friendly_network_error(e: reqwest::Error) -> String {
    if e.is_connect() {
        "Could not reach the server — check your server URL and internet connection.".into()
    } else if e.is_timeout() {
        "Connection timed out — the server may be down.".into()
    } else {
        format!("Network error: {e}")
    }
}

/// Extract a human-readable `message` from an API JSON error body.
/// Falls back to a generic description based on the HTTP status code.
pub(crate) fn friendly_api_error(status: reqwest::StatusCode, body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(msg) = v["message"].as_str() {
            return msg.to_owned();
        }
    }
    match status.as_u16() {
        400 => "Bad request — please check your input.".into(),
        401 => "Invalid username or password.".into(),
        403 => "You don't have permission to do that.".into(),
        404 => "Not found.".into(),
        409 => "That already exists.".into(),
        429 => "Too many requests — please slow down.".into(),
        500..=599 => "Server error — please try again later.".into(),
        _ => format!("Request failed (HTTP {status})."),
    }
}

/// Build a pre-configured reqwest client for one API call.
pub(crate) fn api_client(session: &Session) -> anyhow::Result<(Client, String)> {
    let token = session
        .access_token
        .as_deref()
        .unwrap_or("");

    let mut headers = HeaderMap::new();
    if !token.is_empty() {
        let bearer = format!("Bearer {}", token);
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&bearer)?);
    }
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = Client::builder()
        .default_headers(headers)
        .build()?;

    let base = session
        .server_url
        .trim_end_matches('/')
        .to_owned();

    Ok((client, base))
}
