//! Domain types matching the Nexus server models (snake_case field names).

use serde::{Deserialize, Serialize};

// ── Bot application ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotApplication {
    pub id: String,
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotToken {
    pub token: String,
    pub bot_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotServerInstall {
    pub server_id: String,
    pub bot_user_id: String,
    pub application_id: String,
    pub permissions: i64,
}

// ── Slash commands ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandType(pub u8);

impl CommandType {
    pub const CHAT_INPUT: Self = Self(1);
    pub const USER: Self = Self(2);
    pub const MESSAGE: Self = Self(3);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChoiceValue {
    String(String),
    Integer(i64),
    Float(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandChoice {
    pub name: String,
    pub value: ChoiceValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOption {
    #[serde(rename = "type")]
    pub kind: u8,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub choices: Vec<CommandChoice>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub options: Vec<CommandOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    pub id: String,
    pub application_id: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "type", default = "default_chat_input")]
    pub kind: u8,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub options: Vec<CommandOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_member_permissions: Option<String>,
    #[serde(default = "default_true")]
    pub dm_permission: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<String>,
}

fn default_chat_input() -> u8 { 1 }
fn default_true() -> bool { true }

// ── Interactions ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InteractionType(pub u8);

impl InteractionType {
    pub const PING: Self = Self(1);
    pub const APPLICATION_COMMAND: Self = Self(2);
    pub const MESSAGE_COMPONENT: Self = Self(3);
    pub const AUTOCOMPLETE: Self = Self(4);
    pub const MODAL_SUBMIT: Self = Self(5);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub id: String,
    pub application_id: String,
    #[serde(rename = "type")]
    pub kind: InteractionType,
    pub token: String,
    #[serde(default = "default_version")]
    pub version: u8,
    pub data: Option<serde_json::Value>,
    pub guild_id: Option<String>,
    pub channel_id: Option<String>,
    pub user_id: Option<String>,
}

fn default_version() -> u8 { 1 }

// ── Webhooks ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedFooter {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedImage {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedAuthor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Embed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<EmbedFooter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<EmbedImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<EmbedImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<EmbedAuthor>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fields: Vec<EmbedField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: u8,
    pub server_id: Option<String>,
    pub channel_id: Option<String>,
    pub name: String,
    pub token: Option<String>,
    pub avatar: Option<String>,
    pub application_id: Option<String>,
}
