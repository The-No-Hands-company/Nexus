//! Gaming overlay — a transparent always-on-top window showing voice participants.
//!
//! The overlay window is pre-created in tauri.conf.json (label "overlay") with:
//!   - `decorations: false`
//!   - `transparent: true`
//!   - `alwaysOnTop: true`
//!   - `skipTaskbar: true`
//!   - `visible: false`
//!
//! It renders a compact voice-participant list via the frontend overlay route.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::state::AppState;

/// Participant shown in the overlay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayParticipant {
    pub user_id: String,
    pub username: String,
    pub speaking: bool,
    pub muted: bool,
    pub deafened: bool,
    pub avatar: Option<String>,
}

/// Tauri command: show the overlay window and update position if provided.
#[tauri::command]
pub async fn show_overlay(
    app: AppHandle,
    x: Option<i32>,
    y: Option<i32>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("overlay")
        .ok_or("Overlay window not found")?;

    if let (Some(x), Some(y)) = (x, y) {
        window
            .set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
            .map_err(|e| e.to_string())?;
    }

    window.show().map_err(|e| e.to_string())?;
    window.set_always_on_top(true).map_err(|e| e.to_string())?;

    *app.state::<AppState>().overlay_visible.lock().unwrap() = true;
    Ok(())
}

/// Tauri command: hide the overlay window.
#[tauri::command]
pub async fn hide_overlay(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("overlay") {
        window.hide().map_err(|e| e.to_string())?;
    }
    *app.state::<AppState>().overlay_visible.lock().unwrap() = false;
    Ok(())
}

/// Tauri command: push updated participant list to the overlay.
///
/// Called periodically by the main window as voice state changes.
#[tauri::command]
pub fn update_overlay_participants(
    app: AppHandle,
    participants: Vec<OverlayParticipant>,
) -> Result<(), String> {
    app.emit("overlay-participants", &participants)
        .map_err(|e| e.to_string())
}

/// Show overlay automatically when joining a voice channel (called from gateway event handler).
pub fn auto_show_on_voice_join<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.show();
        let _ = window.set_always_on_top(true);
        *app.state::<AppState>().overlay_visible.lock().unwrap() = true;
    }
}

/// Hide overlay when leaving voice.
pub fn auto_hide_on_voice_leave<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.hide();
        *app.state::<AppState>().overlay_visible.lock().unwrap() = false;
    }
}
