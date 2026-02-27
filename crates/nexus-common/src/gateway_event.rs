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
    // v0.12 — Channel type completion
    /// A new forum post (thread) was created in a forum channel.
    pub const FORUM_POST_CREATE: &str = "FORUM_POST_CREATE";
    /// A forum post's title or tags were updated.
    pub const FORUM_POST_UPDATE: &str = "FORUM_POST_UPDATE";
    /// A forum post was deleted.
    pub const FORUM_POST_DELETE: &str = "FORUM_POST_DELETE";
    /// A stage instance was created on a stage channel.
    pub const STAGE_INSTANCE_CREATE: &str = "STAGE_INSTANCE_CREATE";
    /// A stage instance's topic or privacy was updated.
    pub const STAGE_INSTANCE_UPDATE: &str = "STAGE_INSTANCE_UPDATE";
    /// A stage instance ended.
    pub const STAGE_INSTANCE_DELETE: &str = "STAGE_INSTANCE_DELETE";
    /// The speaker list or hand-raised list on a stage changed.
    pub const STAGE_SPEAKER_UPDATE: &str = "STAGE_SPEAKER_UPDATE";
    /// A user was added to a group DM.
    pub const CHANNEL_RECIPIENT_ADD: &str = "CHANNEL_RECIPIENT_ADD";
    /// A user was removed from a group DM.
    pub const CHANNEL_RECIPIENT_REMOVE: &str = "CHANNEL_RECIPIENT_REMOVE";
    // v0.13 — Engagement Features
    /// A new poll was created in a channel.
    pub const POLL_CREATE: &str = "POLL_CREATE";
    /// A vote was cast on a poll option.
    pub const POLL_VOTE_ADD: &str = "POLL_VOTE_ADD";
    /// A vote was retracted from a poll option.
    pub const POLL_VOTE_REMOVE: &str = "POLL_VOTE_REMOVE";
    /// A poll reached its end time or was ended manually.
    pub const POLL_ENDED: &str = "POLL_ENDED";
    /// A scheduled message was dispatched and created in the channel.
    pub const SCHEDULED_MESSAGE_SENT: &str = "SCHEDULED_MESSAGE_SENT";
    // v0.14 — Platform Differentiation
    /// A message was forwarded to one or more target channels.
    pub const MESSAGE_FORWARD: &str = "MESSAGE_FORWARD";
    /// A new server event was created.
    pub const GUILD_SCHEDULED_EVENT_CREATE: &str = "GUILD_SCHEDULED_EVENT_CREATE";
    /// A server event was updated.
    pub const GUILD_SCHEDULED_EVENT_UPDATE: &str = "GUILD_SCHEDULED_EVENT_UPDATE";
    /// A server event was cancelled or completed.
    pub const GUILD_SCHEDULED_EVENT_DELETE: &str = "GUILD_SCHEDULED_EVENT_DELETE";
    /// A user RSVPed to a server event.
    pub const GUILD_SCHEDULED_EVENT_USER_ADD: &str = "GUILD_SCHEDULED_EVENT_USER_ADD";
    /// A user removed their RSVP from a server event.
    pub const GUILD_SCHEDULED_EVENT_USER_REMOVE: &str = "GUILD_SCHEDULED_EVENT_USER_REMOVE";
    /// A server event became active (start time reached).
    pub const GUILD_SCHEDULED_EVENT_START: &str = "GUILD_SCHEDULED_EVENT_START";
    /// A new sticker was uploaded to a server.
    pub const GUILD_STICKERS_UPDATE: &str = "GUILD_STICKERS_UPDATE";
    /// A stream-mode channel topic bar was updated.
    pub const STREAM_TOPIC_UPDATE: &str = "STREAM_TOPIC_UPDATE";
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
