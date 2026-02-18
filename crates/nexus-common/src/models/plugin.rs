//! Client plugin & custom theme models.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ============================================================================
// Plugins
// ============================================================================

/// A client-side plugin available in the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientPlugin {
    pub id: Uuid,
    pub author_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub version: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub engine_range: String,
    pub permissions: Vec<String>,
    pub bundle_url: Option<String>,
    pub bundle_hash: Option<String>,
    pub verified: bool,
    pub active: bool,
    pub install_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Submit a new plugin to the marketplace.
#[derive(Debug, Deserialize)]
pub struct SubmitPluginRequest {
    pub name: String,
    pub slug: String,
    pub version: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub engine_range: Option<String>,
    /// Permissions this plugin requires (e.g. ["read_messages", "send_messages"])
    pub permissions: Vec<String>,
    pub bundle_url: String,
    pub bundle_hash: String,
}

/// A plugin installed by a user (with per-user settings).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPluginInstall {
    pub user_id: Uuid,
    pub plugin_id: Uuid,
    pub enabled: bool,
    pub settings: serde_json::Value,
    pub installed_at: DateTime<Utc>,
    /// Populated when listing user plugins
    pub plugin: Option<ClientPlugin>,
}

/// Update user settings for an installed plugin.
#[derive(Debug, Deserialize)]
pub struct UpdatePluginSettingsRequest {
    pub enabled: Option<bool>,
    pub settings: Option<serde_json::Value>,
}

// ============================================================================
// Themes
// ============================================================================

/// A custom theme available in the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: Uuid,
    pub author_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub version: String,
    pub description: Option<String>,
    /// CSS variable overrides: `{"--nexus-accent": "#7c6af7"}`
    pub variables: serde_json::Value,
    /// Raw CSS injected after variables are applied.
    pub css: String,
    pub preview_url: Option<String>,
    pub verified: bool,
    pub active: bool,
    pub install_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Submit a new theme to the marketplace.
#[derive(Debug, Deserialize)]
pub struct SubmitThemeRequest {
    pub name: String,
    pub slug: String,
    pub version: String,
    pub description: Option<String>,
    pub variables: serde_json::Value,
    pub css: String,
    pub preview_url: Option<String>,
}

/// A theme installed by a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserThemeInstall {
    pub user_id: Uuid,
    pub theme_id: Uuid,
    pub active: bool,
    pub installed_at: DateTime<Utc>,
    /// Populated when listing user themes
    pub theme: Option<Theme>,
}
