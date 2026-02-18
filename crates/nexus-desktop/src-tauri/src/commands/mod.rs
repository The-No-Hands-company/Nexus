//! Tauri commands — the bridge between the React frontend and the Nexus REST API.
//!
//! Each sub-module groups commands by domain, mirroring the server-side route structure.
//! All HTTP calls use the `reqwest` client with the base URL from [`AppState::session`].

pub mod auth;
pub mod channels;
pub mod e2ee;
pub mod messages;
pub mod presence;
pub mod servers;
pub mod settings;
pub mod voice;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;

use crate::state::Session;

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
