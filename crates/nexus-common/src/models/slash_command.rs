//! Slash command & interaction models.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Application command type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandType {
    ChatInput = 1,
    User = 2,
    Message = 3,
}

/// Option type for slash command parameters.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum OptionType {
    SubCommand = 1,
    SubCommandGroup = 2,
    String = 3,
    Integer = 4,
    Boolean = 5,
    User = 6,
    Channel = 7,
    Role = 8,
    Mentionable = 9,
    Number = 10,
    Attachment = 11,
}

/// A choice for a String/Integer/Number option.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandChoice {
    pub name: String,
    pub value: serde_json::Value,
}

/// A command option (parameter or subcommand).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOption {
    #[serde(rename = "type")]
    pub option_type: OptionType,
    pub name: String,
    pub description: String,
    pub required: Option<bool>,
    pub choices: Option<Vec<CommandChoice>>,
    pub options: Option<Vec<CommandOption>>,
    pub min_value: Option<serde_json::Value>,
    pub max_value: Option<serde_json::Value>,
    pub autocomplete: Option<bool>,
}

/// A registered slash command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    pub id: Uuid,
    pub application_id: Uuid,
    pub server_id: Option<Uuid>,
    pub name: String,
    pub name_localizations: Option<serde_json::Value>,
    pub description: String,
    pub description_localizations: Option<serde_json::Value>,
    pub options: Vec<CommandOption>,
    pub default_member_permissions: Option<String>,
    pub dm_permission: bool,
    pub command_type: i32,
    pub version: Uuid,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Register or update a slash command.
#[derive(Debug, Deserialize)]
pub struct UpsertCommandRequest {
    pub name: String,
    pub description: String,
    pub options: Option<Vec<CommandOption>>,
    pub default_member_permissions: Option<String>,
    pub dm_permission: Option<bool>,
    pub command_type: Option<i32>,
    pub name_localizations: Option<serde_json::Value>,
    pub description_localizations: Option<serde_json::Value>,
}

/// Interaction data sent from client to bot via the interactions endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub id: Uuid,
    pub application_id: Uuid,
    pub interaction_type: String,
    pub data: Option<serde_json::Value>,
    pub server_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub user_id: Uuid,
    pub token: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Resolved interaction option value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionOption {
    pub name: String,
    pub value: Option<serde_json::Value>,
    pub options: Option<Vec<InteractionOption>>,
    pub focused: Option<bool>,
}

/// Create an interaction (called internally when user invokes a slash command).
#[derive(Debug, Deserialize)]
pub struct CreateInteractionRequest {
    pub interaction_type: String,
    pub command_id: Option<Uuid>,
    pub data: serde_json::Value,
}

/// Respond to an interaction (called by the bot).
#[derive(Debug, Deserialize)]
pub struct InteractionResponse {
    /// 1=PONG, 4=CHANNEL_MESSAGE_WITH_SOURCE, 5=DEFERRED_RESPONSE,
    /// 6=DEFERRED_UPDATE, 7=UPDATE_MESSAGE, 8=AUTOCOMPLETE, 9=MODAL
    pub response_type: i32,
    pub data: Option<serde_json::Value>,
}
