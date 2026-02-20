//! Global hotkeys — push-to-talk (PTT) and other system-wide shortcuts.
//!
//! PTT default: `CapsLock` (user-configurable via Settings → Voice → PTT key).
//!
//! When the PTT key is pressed:
//!   1. Backend emits `ptt-start` event → frontend enables mic capture.
//! When released:
//!   2. Backend emits `ptt-stop` event → frontend disables mic capture.

use tauri::{App, AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::state::AppState;

const DEFAULT_PTT_SHORTCUT: &str = "CapsLock";

/// Register the default push-to-talk shortcut on application startup.
pub fn register_defaults<R: Runtime>(app: &mut App<R>) -> tauri::Result<()> {
    let handle = app.handle().clone();
    // Unregister anything left over from a previous session before registering.
    // This prevents a panic when the OS still holds the hotkey from a stale process.
    let _ = handle.global_shortcut().unregister_all();
    if let Err(e) = register_ptt_shortcut(&handle, DEFAULT_PTT_SHORTCUT) {
        // Log and continue — PTT simply won't work until the user changes it in Settings.
        tracing::warn!("Could not register PTT shortcut '{}': {}. PTT disabled until reassigned in Settings > Voice.", DEFAULT_PTT_SHORTCUT, e);
    }
    Ok(())
}

/// Register a specific shortcut string as the PTT key.
pub fn register_ptt_shortcut<R: Runtime>(app: &AppHandle<R>, shortcut_str: &str) -> tauri::Result<()> {
    let handle = app.clone();
    let shortcut: Shortcut = shortcut_str.parse().map_err(|e| {
        tauri::Error::Anyhow(anyhow::anyhow!("Invalid shortcut '{}': {}", shortcut_str, e))
    })?;

    // Always clear our own shortcuts first — safe to call even if nothing is registered.
    let _ = app.global_shortcut().unregister_all();

    app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
        match event.state {
            ShortcutState::Pressed => {
                if let Some(state) = handle.try_state::<AppState>() {
                    let mut ptt = state.ptt.lock().unwrap();
                    if !ptt.transmitting {
                        ptt.transmitting = true;
                        let _ = handle.emit("ptt-start", ());
                        tracing::debug!("PTT start");
                    }
                }
            }
            ShortcutState::Released => {
                if let Some(state) = handle.try_state::<AppState>() {
                    let mut ptt = state.ptt.lock().unwrap();
                    ptt.transmitting = false;
                    let _ = handle.emit("ptt-stop", ());
                    tracing::debug!("PTT stop");
                }
            }
        }
    }).map_err(|e| tauri::Error::Anyhow(anyhow::anyhow!("GlobalShortcut error: {}", e)))?;

    // Store the new shortcut string
    if let Some(state) = app.try_state::<AppState>() {
        state.ptt.lock().unwrap().shortcut = shortcut_str.to_owned();
    }

    Ok(())
}

/// Tauri command: change the PTT shortcut at runtime.
///
/// Unregisters the old shortcut, registers the new one, persists to store.
#[tauri::command]
pub fn set_ptt_shortcut(
    app: AppHandle,
    shortcut: String,
) -> Result<(), String> {
    // Unregister all existing shortcuts managed by us
    app.global_shortcut().unregister_all().map_err(|e| e.to_string())?;

    // Register the new one
    register_ptt_shortcut(&app, &shortcut).map_err(|e| e.to_string())?;

    tracing::info!("PTT shortcut changed to '{shortcut}'");
    Ok(())
}

/// Tauri command: return the current PTT shortcut string.
#[tauri::command]
pub fn get_ptt_shortcut(app: AppHandle) -> String {
    let shortcut = app.state::<AppState>()
        .ptt
        .lock()
        .unwrap()
        .shortcut
        .clone();
    if shortcut.is_empty() {
        DEFAULT_PTT_SHORTCUT.to_owned()
    } else {
        shortcut
    }
}
