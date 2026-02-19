//! Auto-update — background check using tauri-plugin-updater.
//!
//! Checks for a new release every 4 hours by default.
//! On finding a pending update, emits `update-available` to the frontend
//! (which renders a non-intrusive banner) instead of updating silently.
//! The user explicitly triggers download + install.

use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;
use tokio::time::{interval, Duration};


/// Spawn a background task that polls for updates on a fixed interval.
///
/// Call from `app.setup()`:
/// ```rust
/// updater::schedule_check(app.handle().clone());
/// ```
pub fn schedule_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // First check after 30 seconds (let the app settle)
        tokio::time::sleep(Duration::from_secs(30)).await;
        check_once(&app).await;

        // Then every 4 hours
        let mut ticker = interval(Duration::from_secs(4 * 60 * 60));
        loop {
            ticker.tick().await;
            check_once(&app).await;
        }
    });
}

/// Perform a single update check; emit an event if an update is available.
async fn check_once(app: &AppHandle) {
    tracing::debug!("Checking for updates...");
    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => {
                tracing::info!(
                    "Update available: {} → {}",
                    update.current_version,
                    update.version
                );
                let _ = app.emit(
                    "update-available",
                    serde_json::json!({
                        "current_version": update.current_version,
                        "new_version": update.version,
                        "body": update.body,
                        "date": update.date,
                    }),
                );
            }
            Ok(None) => {
                tracing::debug!("No update available");
            }
            Err(e) => {
                tracing::warn!("Update check failed: {e}");
            }
        },
        Err(e) => {
            tracing::warn!("Updater not available: {e}");
        }
    }
}
