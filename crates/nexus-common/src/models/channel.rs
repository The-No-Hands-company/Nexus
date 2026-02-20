//! Channel model — where conversation happens.
//!
//! Nexus channels improve on Discord's model with:
//! - Proper thread support (not bolted on)
//! - Per-channel notification granularity
//! - Channel-level E2EE opt-in
//! - Forum-style channels (first-class, not an afterthought)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// A channel within a server or a DM conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Uuid,

    /// Server this channel belongs to (None for DMs)
    pub server_id: Option<Uuid>,

    /// Parent channel ID (for threads, sub-channels)
    pub parent_id: Option<Uuid>,

    /// Channel type
    pub channel_type: ChannelType,

    /// Channel name (required for server channels)
    pub name: Option<String>,

    /// Channel topic / description
    pub topic: Option<String>,

    /// Position in the channel list (for ordering)
    pub position: i32,

    /// Is this channel NSFW-marked (server admin toggle, NO mandatory ID check)
    pub nsfw: bool,

    /// Slowmode delay in seconds (0 = off)
    pub rate_limit_per_user: i32,

    /// Bitrate for voice channels (in bits/sec)
    pub bitrate: Option<i32>,

    /// User limit for voice channels (0 = unlimited)
    pub user_limit: Option<i32>,

    /// Whether E2EE is enabled for this channel
    pub encrypted: bool,

    /// Channel-specific permission overrides (JSON)
    pub permission_overwrites: serde_json::Value,

    /// Last message ID for read-state tracking
    pub last_message_id: Option<Uuid>,

    /// Auto-archive duration for threads (minutes)
    pub auto_archive_duration: Option<i32>,

    /// Whether the thread is archived
    pub archived: bool,

    /// Whether the thread is locked (no new messages)
    pub locked: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// Standard text channel in a server
    Text,
    /// Voice channel (with optional text chat)
    Voice,
    /// Category (organizer for other channels)
    Category,
    /// Direct message (1:1)
    Dm,
    /// Group DM (2-10 users)
    GroupDm,
    /// Thread (spawned from a message)
    Thread,
    /// Forum channel (topic-based, each "post" is a thread)
    Forum,
    /// Stage channel (one-to-many broadcasting)
    Stage,
    /// Announcement channel (crosspost-able)
    Announcement,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateChannelRequest {
    #[validate(length(min = 1, max = 100, message = "Channel name must be 1-100 characters"))]
    pub name: String,

    pub channel_type: ChannelType,

    #[validate(length(max = 1024))]
    pub topic: Option<String>,

    pub parent_id: Option<Uuid>,

    pub position: Option<i32>,

    pub nsfw: Option<bool>,

    pub bitrate: Option<i32>,

    pub user_limit: Option<i32>,

    pub rate_limit_per_user: Option<i32>,

    /// Enable E2E encryption for this channel
    pub encrypted: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateChannelRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    #[validate(length(max = 1024))]
    pub topic: Option<String>,

    pub position: Option<i32>,

    pub nsfw: Option<bool>,

    pub rate_limit_per_user: Option<i32>,

    pub bitrate: Option<i32>,

    pub user_limit: Option<i32>,

    pub parent_id: Option<Uuid>,
}
