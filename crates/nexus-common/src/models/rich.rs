//! Thread and custom emoji models for v0.4 Rich Features.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// ============================================================
// Threads
// ============================================================

/// A thread — a focused conversation spawned from a message.
/// Threads are backed by a `channels` record (channel_type = 'thread')
/// plus a row in the `threads` table for thread-specific metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// The channel ID for this thread
    pub id: Uuid,

    /// The channel this thread lives in
    pub parent_channel_id: Uuid,

    /// Message that spawned this thread (optional for forum channels)
    pub parent_message_id: Option<Uuid>,

    /// User who created the thread
    pub owner_id: Uuid,

    /// Thread title
    pub title: String,

    pub message_count: i32,
    pub member_count: i32,

    /// Auto-archive after N minutes of inactivity
    pub auto_archive_minutes: i32,

    pub archived: bool,
    pub archived_at: Option<DateTime<Utc>>,
    pub locked: bool,

    /// Optional forum-style tags
    pub tags: Vec<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row returned from the `threads` table join.
#[derive(Debug)]
pub struct ThreadRow {
    pub channel_id: Uuid,
    pub parent_message_id: Option<Uuid>,
    pub owner_id: Uuid,
    pub title: String,
    pub message_count: i32,
    pub member_count: i32,
    pub auto_archive_minutes: i32,
    pub archived: bool,
    pub archived_at: Option<DateTime<Utc>>,
    pub locked: bool,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Joined from channels
    pub parent_channel_id: Option<Uuid>,
}

/// Create thread request.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateThreadRequest {
    #[validate(length(min = 1, max = 100, message = "Thread title must be 1-100 characters"))]
    pub title: String,

    /// Source message (required for message-threads, optional for forum posts)
    pub message_id: Option<Uuid>,

    /// Auto-archive threshold in minutes (60, 1440, 4320, or 10080)
    pub auto_archive_minutes: Option<i32>,

    /// Optional forum tags
    pub tags: Option<Vec<String>>,
}

/// Update thread settings request.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateThreadRequest {
    #[validate(length(min = 1, max = 100))]
    pub title: Option<String>,
    pub archived: Option<bool>,
    pub locked: Option<bool>,
    pub auto_archive_minutes: Option<i32>,
    pub tags: Option<Vec<String>>,
}

// ============================================================
// Custom Emoji
// ============================================================

/// A custom server emoji (uploaded by server members with permission).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEmoji {
    pub id: Uuid,
    pub server_id: Uuid,
    pub creator_id: Option<Uuid>,

    /// Short name used in `:name:` syntax
    pub name: String,

    /// Public URL for display
    pub url: Option<String>,

    /// Whether this emoji is animated (GIF/WebP)
    pub animated: bool,
    pub managed: bool,
    pub available: bool,

    pub created_at: DateTime<Utc>,
}

/// Row from the `server_emoji` table.
#[derive(Debug)]
pub struct ServerEmojiRow {
    pub id: Uuid,
    pub server_id: Uuid,
    pub creator_id: Option<Uuid>,
    pub name: String,
    pub storage_key: String,
    pub url: Option<String>,
    pub animated: bool,
    pub managed: bool,
    pub available: bool,
    pub created_at: DateTime<Utc>,
}

impl From<ServerEmojiRow> for ServerEmoji {
    fn from(r: ServerEmojiRow) -> Self {
        Self {
            id: r.id,
            server_id: r.server_id,
            creator_id: r.creator_id,
            name: r.name,
            url: r.url,
            animated: r.animated,
            managed: r.managed,
            available: r.available,
            created_at: r.created_at,
        }
    }
}

/// Create custom emoji request.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateEmojiRequest {
    #[validate(length(min = 2, max = 32, message = "Emoji name must be 2-32 characters"))]
    #[validate(regex(
        path = *EMOJI_NAME_REGEX,
        message = "Emoji name can only contain letters, numbers, and underscores"
    ))]
    pub name: String,
}

/// Update emoji request.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateEmojiRequest {
    #[validate(length(min = 2, max = 32))]
    pub name: Option<String>,
}

// ============================================================
// Attachment
// ============================================================

/// Row from the `attachments` table.
#[derive(Debug)]
pub struct AttachmentRow {
    pub id: Uuid,
    pub uploader_id: Uuid,
    pub server_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub storage_key: String,
    pub url: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_secs: Option<f64>,
    pub spoiler: bool,
    pub blurhash: Option<String>,
    pub sha256: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Enhanced presence / user activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivity {
    pub user_id: Uuid,
    pub activity_type: Option<String>,
    pub name: Option<String>,
    pub details: Option<String>,
    pub state: Option<String>,
    pub large_image: Option<String>,
    pub small_image: Option<String>,
    pub url: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

/// Update presence / activity request.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePresenceRequest {
    /// Presence state
    pub presence: Option<crate::models::user::UserPresence>,

    /// Custom status text
    #[validate(length(max = 128))]
    pub status: Option<String>,

    /// Custom status emoji short-code or unicode
    #[validate(length(max = 64))]
    pub custom_status_emoji: Option<String>,

    /// Activity update (game, music, streaming)
    pub activity: Option<ActivityUpdate>,
}

#[derive(Debug, Deserialize)]
pub struct ActivityUpdate {
    pub activity_type: Option<String>,
    pub name: Option<String>,
    pub details: Option<String>,
    pub state: Option<String>,
    pub url: Option<String>,
    pub large_image: Option<String>,
    pub small_image: Option<String>,
}

use std::sync::LazyLock;
static EMOJI_NAME_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^[a-zA-Z0-9_]+$").unwrap());
