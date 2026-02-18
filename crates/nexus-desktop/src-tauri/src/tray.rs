//! System tray — presence menu, quick actions, show/hide main window.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Emitter, Manager, Runtime,
};

/// Build and attach the system tray icon + context menu.
pub fn setup_tray<R: Runtime>(app: &mut App<R>) -> tauri::Result<()> {
    let handle = app.handle();

    // ── Menu items ──────────────────────────────────────────────────────────
    let show = MenuItem::with_id(handle, "show", "Show Nexus", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(handle)?;

    let online = MenuItem::with_id(handle, "presence_online", "● Online", true, None::<&str>)?;
    let idle = MenuItem::with_id(handle, "presence_idle", "◑ Idle", true, None::<&str>)?;
    let dnd = MenuItem::with_id(handle, "presence_dnd", "⊘ Do Not Disturb", true, None::<&str>)?;
    let invisible =
        MenuItem::with_id(handle, "presence_invisible", "○ Invisible", true, None::<&str>)?;

    let separator2 = PredefinedMenuItem::separator(handle)?;
    let quit = MenuItem::with_id(handle, "quit", "Quit Nexus", true, None::<&str>)?;

    let menu = Menu::with_items(
        handle,
        &[
            &show,
            &separator,
            &online,
            &idle,
            &dnd,
            &invisible,
            &separator2,
            &quit,
        ],
    )?;

    // ── Tray icon ───────────────────────────────────────────────────────────
    TrayIconBuilder::with_id("main-tray")
        .tooltip("Nexus")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            let handle = app.clone();
            match event.id.as_ref() {
                "show" => {
                    if let Some(window) = handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    std::process::exit(0);
                }
                id if id.starts_with("presence_") => {
                    let presence = id.trim_start_matches("presence_");
                    // Emit to frontend so it can call the API
                    let _ = handle.emit("tray-presence-change", presence.to_owned());
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            // Left-click toggles main window visibility
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(handle)?;

    Ok(())
}

/// Update the tray tooltip to show current presence/server.
pub fn update_tray_tooltip<R: Runtime>(
    app: &tauri::AppHandle<R>,
    text: &str,
) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        tray.set_tooltip(Some(text))?;
    }
    Ok(())
}
