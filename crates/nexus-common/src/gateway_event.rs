//! Gateway event types — shared between API and Gateway crates.
//!
//! The API emits events when data changes (message created, member joined, etc.)
//! and the Gateway forwards them to connected WebSocket clients.
//! This module lives in `nexus-common` so both crates can use it without circular deps.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
