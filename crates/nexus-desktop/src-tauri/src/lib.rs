//! Nexus Desktop — Tauri 2 application entry point.
//!
//! Responsibilities of the Tauri backend:
//! - Manage persistent user state (credentials, settings) via tauri-plugin-store
//! - Broker HTTP calls to the Nexus API (avoids CORS and manages auth tokens)
//! - Maintain a persistent WebSocket connection to the gateway
//! - Expose Tauri commands consumed by the React frontend
//! - System tray with presence/quick-action menu
//! - Push-to-talk global hotkey
//! - Gaming overlay window
//! - Auto-update checks

pub mod commands;
pub mod hotkeys;
pub mod notifications;
pub mod overlay;
pub mod state;
pub mod tray;
pub mod updater;

use tracing_subscriber::{fmt, EnvFilter};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise structured logging
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        // ── Plugins ──────────────────────────────────────────────────────────
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_os::init())
        // ── App state ────────────────────────────────────────────────────────
        .manage(state::AppState::default())
        // ── Setup hook ───────────────────────────────────────────────────────
        .setup(|app| {
            // System tray
            tray::setup_tray(app)?;

            // Register push-to-talk shortcut (default: CapsLock, user-configurable)
            hotkeys::register_defaults(app)?;

            // Start background update check (every 4 hours)
            updater::schedule_check(app.handle().clone());

            tracing::info!("Nexus desktop v{} ready", env!("CARGO_PKG_VERSION"));
            Ok(())
        })
        // ── Tauri commands ───────────────────────────────────────────────────
        .invoke_handler(tauri::generate_handler![
            // Auth
            commands::auth::login,
            commands::auth::logout,
            commands::auth::refresh_token,
            commands::auth::get_current_user,
            // Servers & channels
            commands::servers::list_servers,
            commands::servers::get_server,
            commands::channels::list_channels,
            commands::channels::get_channel,
            // Messages
            commands::messages::send_message,
            commands::messages::fetch_history,
            // Encrypted messaging
            commands::e2ee::send_encrypted_message,
            commands::e2ee::fetch_encrypted_history,
            commands::e2ee::register_device,
            commands::e2ee::get_key_bundle,
            // Presence & voice
            commands::presence::update_presence,
            commands::voice::get_voice_state,
            // Settings & window management
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::set_server_url,
            // Overlay
            overlay::show_overlay,
            overlay::hide_overlay,
            overlay::update_overlay_participants,
            // Notifications
            notifications::show_notification,
            // Hotkeys
            hotkeys::set_ptt_shortcut,
            hotkeys::get_ptt_shortcut,
        ])
        .on_window_event(|window, event| {
            // Intercept close on main window → minimise to tray instead
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Nexus desktop application");
}
