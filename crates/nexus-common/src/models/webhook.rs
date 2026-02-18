//! Webhook models.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Webhook type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WebhookType {
    Incoming,
    Outgoing,
}

impl std::fmt::Display for WebhookType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Incoming => write!(f, "incoming"),
            Self::Outgoing => write!(f, "outgoing"),
        }
    }
}

/// A webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: Uuid,
    pub webhook_type: WebhookType,
    pub server_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub creator_id: Option<Uuid>,
    pub name: String,
    pub avatar: Option<String>,
    /// Token is only included when the webhook is first created or when
    /// queried by the creator — never exposed to third parties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    pub url: Option<String>,
    pub events: Vec<String>,
    pub active: bool,
    pub delivery_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new incoming webhook (for a channel).
#[derive(Debug, Deserialize)]
pub struct CreateIncomingWebhookRequest {
    pub name: String,
    pub avatar: Option<String>,
}

/// Create a new outgoing webhook (fires HTTP POST on events).
#[derive(Debug, Deserialize)]
pub struct CreateOutgoingWebhookRequest {
    pub name: String,
    pub url: String,
    /// Gateway event names to subscribe to, e.g. ["MESSAGE_CREATE"]
    pub events: Vec<String>,
    pub avatar: Option<String>,
}

/// Modify an existing webhook.
#[derive(Debug, Deserialize)]
pub struct ModifyWebhookRequest {
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub channel_id: Option<Uuid>,
    pub url: Option<String>,
    pub events: Option<Vec<String>>,
    pub active: Option<bool>,
}

/// Execute an incoming webhook — post a message to the channel.
#[derive(Debug, Deserialize)]
pub struct ExecuteWebhookRequest {
    pub content: Option<String>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub embeds: Option<Vec<serde_json::Value>>,
    pub allowed_mentions: Option<serde_json::Value>,
    /// Optional thread ID to post into.
    pub thread_id: Option<Uuid>,
}

/// Delivery status of an outgoing webhook fire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub webhook_id: Uuid,
    pub event_type: String,
    pub status_code: Option<i32>,
    pub success: bool,
    pub fired_at: DateTime<Utc>,
}
