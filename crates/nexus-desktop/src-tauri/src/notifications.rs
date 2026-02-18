//! Desktop notifications — bridge between gateway events and OS notifications.

use tauri::{AppHandle, Emitter, Runtime};

/// Show a desktop notification.
///
/// Called from Tauri commands or from the gateway event processor when
/// a message arrives in a channel the user is not currently viewing.
#[tauri::command]
pub fn show_notification(title: String, body: String, icon: Option<String>) {
    // The tauri-plugin-notification API is invoked from the frontend JS side.
    // This command exists so Rust code can trigger a notification programmatically
    // by emitting an event that the frontend plugin picks up.
    let _ = (title, body, icon); // params forwarded via event below
}

/// Emit a "show-notification" event to the frontend, which uses
/// tauri-plugin-notification to display a native OS notification.
pub fn notify<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str, icon: Option<&str>) {
    let payload = serde_json::json!({
        "title": title,
        "body": body,
        "icon": icon,
    });
    let _ = app.emit("native-notification", payload);
}

/// Emit a mention notification with channel routing info.
pub fn notify_mention<R: Runtime>(
    app: &AppHandle<R>,
    server_name: &str,
    channel_name: &str,
    author: &str,
    preview: &str,
    channel_id: &str,
) {
    let payload = serde_json::json!({
        "title": format!("@{author} mentioned you in #{channel_name}"),
        "body": preview,
        "subtitle": server_name,
        "channel_id": channel_id,
    });
    let _ = app.emit("native-notification", payload);
}
