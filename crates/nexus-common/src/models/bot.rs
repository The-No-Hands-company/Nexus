//! Bot application models.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// A bot application (the "app" behind a bot user).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotApplication {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub avatar: Option<String>,
    pub public_key: String,
    pub redirect_uris: Vec<String>,
    pub permissions: i64,
    pub verified: bool,
    pub is_public: bool,
    pub interactions_endpoint_url: Option<String>,
    pub flags: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new bot application.
#[derive(Debug, Deserialize)]
pub struct CreateBotRequest {
    pub name: String,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub redirect_uris: Option<Vec<String>>,
    pub interactions_endpoint_url: Option<String>,
}

/// Update an existing bot application.
#[derive(Debug, Deserialize)]
pub struct UpdateBotRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub avatar: Option<String>,
    pub is_public: Option<bool>,
    pub redirect_uris: Option<Vec<String>>,
    pub interactions_endpoint_url: Option<String>,
}

/// Returned when a bot token is regenerated (shown once).
#[derive(Debug, Serialize)]
pub struct BotToken {
    pub token: String,
}

/// A bot installed in a server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotServerInstall {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub server_id: Uuid,
    pub installed_by: Uuid,
    pub scopes: Vec<String>,
    pub permissions: i64,
    pub installed_at: DateTime<Utc>,
}
