//! Application state — shared across all Tauri commands via `State<AppState>`.

use std::sync::Mutex;
use uuid::Uuid;

/// Credentials stored in memory for the session.
/// Persisted to tauri-plugin-store between restarts.
#[derive(Debug, Default, Clone)]
pub struct Session {
    /// Ecosystem session token (`nxs_…`), presented to apps as a cookie.
    ///
    /// Not a JWT this app minted or can mint. A desktop client is not behind
    /// the ecosystem proxy and has no browser cookie jar, so it does by hand
    /// what a browser does automatically: sign in to Auth, hold the session,
    /// and send it as `nexus_session` on every request. The proxy exchanges it
    /// for a short-lived signed identity per host.
    pub session_token: Option<String>,
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
    /// Base URL of the connected Nexus server (e.g. "https://nexus.chat")
    pub server_url: String,
}

/// Whether push-to-talk is currently held down.
#[derive(Debug, Default)]
pub struct PttState {
    pub transmitting: bool,
    pub shortcut: String, // e.g. "CapsLock"
}

/// Shared application state injected via `tauri::Manager::manage`.
#[derive(Debug, Default)]
pub struct AppState {
    pub session: Mutex<Session>,
    pub ptt: Mutex<PttState>,
    pub overlay_visible: Mutex<bool>,
}

impl AppState {
    /// Convenience: clone session for use in async contexts.
    pub fn session_snapshot(&self) -> Session {
        self.session.lock().unwrap().clone()
    }
}
