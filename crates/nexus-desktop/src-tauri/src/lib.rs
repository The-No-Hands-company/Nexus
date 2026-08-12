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

#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::default_trait_access)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::format_push_string)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::pedantic)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::ref_option)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::wildcard_imports)]

pub mod commands;
pub mod hotkeys;
pub mod notifications;
pub mod overlay;
pub mod state;
pub mod tray;
pub mod updater;

use tracing_subscriber::{EnvFilter, fmt};

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

            // Start background update check only outside debug/dev builds
            if !cfg!(debug_assertions) {
                updater::schedule_check(app.handle().clone());
            } else {
                tracing::info!("Dev build: updater check disabled");
            }

            tracing::info!("Nexus desktop v{} ready", env!("CARGO_PKG_VERSION"));
            Ok(())
        })
        // ── Tauri commands ───────────────────────────────────────────────────
        .invoke_handler(tauri::generate_handler![
            // Auth
            commands::auth::request_access_url,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::get_current_user,
            // Servers & channels
            commands::servers::list_servers,
            commands::servers::get_server,
            commands::servers::create_server,
            commands::servers::update_server,
            commands::servers::delete_server,
            commands::servers::transfer_server_ownership,
            commands::servers::leave_server,
            commands::servers::list_roles,
            commands::servers::create_role,
            commands::servers::update_role,
            commands::servers::delete_role,
            commands::servers::list_server_invites,
            commands::servers::create_invite,
            commands::servers::delete_invite,
            // Emoji management
            commands::emojis::list_emoji,
            commands::emojis::upload_emoji,
            commands::emojis::rename_emoji,
            commands::emojis::delete_emoji,
            // Webhook management
            commands::webhooks::list_webhooks,
            commands::webhooks::create_incoming_webhook,
            commands::webhooks::delete_webhook,
            // Bot management
            commands::bots::list_server_bots,
            commands::bots::install_bot,
            commands::bots::uninstall_bot,
            commands::bots::get_public_bot_info,
            // Boost / supporter tiers
            commands::boosters::get_server_boost_tier,
            commands::boosters::list_server_boosters,
            commands::boosters::boost_server,
            commands::boosters::remove_boost,
            commands::boosters::set_vanity_url,
            // Friends, relationships & DMs
            commands::friends::list_relationships,
            commands::friends::send_friend_request,
            commands::friends::update_relationship,
            commands::friends::delete_relationship,
            commands::friends::search_users,
            commands::friends::list_dm_channels,
            commands::friends::create_dm,
            commands::friends::list_members,
            commands::friends::get_user_profile,
            commands::channels::list_channels,
            commands::channels::get_channel,
            commands::channels::create_channel,
            // Messages
            commands::messages::send_message,
            commands::messages::fetch_history,
            // Encrypted messaging
            commands::e2ee::send_encrypted_message,
            commands::e2ee::fetch_encrypted_history,
            commands::e2ee::register_device,
            commands::e2ee::get_key_bundle,
            commands::e2ee::list_devices,
            commands::e2ee::delete_device,
            // Presence & voice
            commands::presence::update_presence,
            commands::voice::get_voice_state,
            // Settings & window management
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::set_server_url,
            commands::settings::update_profile,
            commands::settings::change_password,
            commands::settings::delete_account,
            commands::settings::cancel_account_deletion,
            // Overlay
            overlay::show_overlay,
            overlay::hide_overlay,
            overlay::update_overlay_participants,
            // Notifications
            notifications::show_notification,
            // Hotkeys
            hotkeys::set_ptt_shortcut,
            hotkeys::get_ptt_shortcut,
            // Server directory / federated room browser
            commands::directory::directory_list_servers,
            commands::directory::directory_list_rooms,
            commands::directory::directory_search_rooms,
            commands::directory::directory_join_room,
        ])
        .on_window_event(|window, event| {
            // Intercept close on main window → minimise to tray instead
            if window.label() == "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Nexus desktop application");
}
