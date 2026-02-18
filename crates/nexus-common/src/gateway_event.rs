//! Gateway event types — shared between API and Gateway crates.
//!
//! The API emits events when data changes (message created, member joined, etc.)
//! and the Gateway forwards them to connected WebSocket clients.
//! This module lives in `nexus-common` so both crates can use it without circular deps.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Well-known event type constants (v0.7 extensibility additions).
pub mod event_types {
    // Existing
    pub const MESSAGE_CREATE: &str = "MESSAGE_CREATE";
    pub const MESSAGE_UPDATE: &str = "MESSAGE_UPDATE";
    pub const MESSAGE_DELETE: &str = "MESSAGE_DELETE";
    pub const TYPING_START: &str = "TYPING_START";
    pub const PRESENCE_UPDATE: &str = "PRESENCE_UPDATE";
    pub const VOICE_STATE_UPDATE: &str = "VOICE_STATE_UPDATE";
    pub const CHANNEL_CREATE: &str = "CHANNEL_CREATE";
    pub const CHANNEL_UPDATE: &str = "CHANNEL_UPDATE";
    pub const CHANNEL_DELETE: &str = "CHANNEL_DELETE";
    pub const SERVER_MEMBER_ADD: &str = "SERVER_MEMBER_ADD";
    pub const SERVER_MEMBER_REMOVE: &str = "SERVER_MEMBER_REMOVE";
    pub const SERVER_MEMBER_UPDATE: &str = "SERVER_MEMBER_UPDATE";
    // v0.7 — Extensibility
    pub const INTERACTION_CREATE: &str = "INTERACTION_CREATE";
    pub const WEBHOOK_EXECUTE: &str = "WEBHOOK_EXECUTE";
    pub const APPLICATION_COMMAND_CREATE: &str = "APPLICATION_COMMAND_CREATE";
    pub const APPLICATION_COMMAND_UPDATE: &str = "APPLICATION_COMMAND_UPDATE";
    pub const APPLICATION_COMMAND_DELETE: &str = "APPLICATION_COMMAND_DELETE";
}

/// Events broadcast through the gateway to connected clients.
///
/// The API creates these when data mutates (REST endpoints), and the gateway
/// forwards them to all connected clients whose subscriptions match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayEvent {
    /// Event type (e.g., "MESSAGE_CREATE", "TYPING_START", "PRESENCE_UPDATE")
    pub event_type: String,
    /// Event payload as JSON
    pub data: serde_json::Value,
    /// Which server this event belongs to (for filtering — only send to members)
    pub server_id: Option<Uuid>,
    /// Which channel this event belongs to
    pub channel_id: Option<Uuid>,
    /// Which user triggered this event
    pub user_id: Option<Uuid>,
}
