//! Message model — the core content unit.
//!
//! Messages in Nexus are stored in ScyllaDB for write-heavy performance
//! and replicated to MeiliSearch for full-text search.
//! Messages support rich formatting, embeds, attachments, reactions, and threads.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// A message in a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,

    /// Channel this message belongs to
    pub channel_id: Uuid,

    /// Author user ID
    pub author_id: Uuid,

    /// Message content (Markdown-flavored, up to configurable limit)
    pub content: String,

    /// Message type
    pub message_type: MessageType,

    /// Whether this message has been edited
    pub edited: bool,

    /// Edit timestamp
    pub edited_at: Option<DateTime<Utc>>,

    /// Whether this message is pinned
    pub pinned: bool,

    /// Embedded content (link previews, rich embeds)
    pub embeds: Vec<Embed>,

    /// File attachments
    pub attachments: Vec<Attachment>,

    /// Emoji reactions
    pub reactions: Vec<Reaction>,

    /// Users/roles mentioned in this message
    pub mentions: Vec<Uuid>,

    /// Roles mentioned
    pub mention_roles: Vec<Uuid>,

    /// Whether @everyone was used
    pub mention_everyone: bool,

    /// Reference to another message (reply, forward)
    pub reference: Option<MessageReference>,

    /// Thread spawned from this message (if any)
    pub thread_id: Option<Uuid>,

    /// If E2EE, this contains the encrypted payload and content is empty
    pub encrypted_content: Option<Vec<u8>>,

    /// Encryption metadata (sender key ID, algorithm, etc.)
    pub encryption_metadata: Option<serde_json::Value>,

    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Normal user message
    Default,
    /// Reply to another message
    Reply,
    /// System message (user joined, channel created, etc.)
    System,
    /// Bot/webhook message
    Bot,
    /// Thread starter message
    ThreadStarter,
    /// Pin notification
    PinNotification,
    /// Member join notification
    MemberJoin,
    /// Server boost notification (if implemented)
    Boost,
}

/// Rich embed — for link previews, bot embeds, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embed {
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub color: Option<u32>,
    pub timestamp: Option<DateTime<Utc>>,
    pub footer: Option<EmbedFooter>,
    pub image: Option<EmbedMedia>,
    pub thumbnail: Option<EmbedMedia>,
    pub video: Option<EmbedMedia>,
    pub author: Option<EmbedAuthor>,
    pub fields: Vec<EmbedField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedFooter {
    pub text: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedMedia {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedAuthor {
    pub name: String,
    pub url: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

/// File attachment metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Uuid,
    /// Original filename
    pub filename: String,
    /// MIME type
    pub content_type: String,
    /// File size in bytes
    pub size: u64,
    /// Storage key (S3/MinIO path)
    pub url: String,
    /// Image/video dimensions
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Whether this is marked as a spoiler
    pub spoiler: bool,
}

/// Emoji reaction on a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    /// Emoji identifier (unicode emoji or custom emoji ID)
    pub emoji: String,
    /// Number of users who reacted with this emoji
    pub count: u32,
    /// Whether the current user (in context) reacted
    pub me: bool,
}

/// Reference to another message (for replies/forwards).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReference {
    pub message_id: Uuid,
    pub channel_id: Uuid,
    pub server_id: Option<Uuid>,
}

/// Create message request.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateMessageRequest {
    #[validate(length(min = 1, max = 4000, message = "Message must be 1-4000 characters"))]
    pub content: String,

    /// Reply to a message
    pub reference: Option<MessageReference>,

    /// Attachment IDs (uploaded separately)
    pub attachment_ids: Option<Vec<Uuid>>,

    /// Whether to suppress embeds
    pub suppress_embeds: Option<bool>,

    /// If channel is E2EE, encrypted content bytes
    pub encrypted_content: Option<Vec<u8>>,
    pub encryption_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateMessageRequest {
    #[validate(length(min = 1, max = 4000))]
    pub content: Option<String>,
}

/// Message search query.
#[derive(Debug, Deserialize)]
pub struct MessageSearchQuery {
    /// Search text
    pub query: String,
    /// Filter by channel
    pub channel_id: Option<Uuid>,
    /// Filter by server
    pub server_id: Option<Uuid>,
    /// Filter by author
    pub author_id: Option<Uuid>,
    /// Filter by date range
    pub before: Option<DateTime<Utc>>,
    pub after: Option<DateTime<Utc>>,
    /// Has attachments/embeds/links
    pub has: Option<Vec<String>>,
    /// Pagination offset
    pub offset: Option<u32>,
    /// Results per page (max 50)
    pub limit: Option<u32>,
}
