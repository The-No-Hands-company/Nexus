//! Phase 16 — Advanced Collaboration & Productivity models.
//!
//! Tasks, calendar events, file versions, and AI-assist preferences.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── 16-01: Tasks & Checklists ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub server_id: Uuid,
    pub channel_id: Uuid,
    pub creator_id: Uuid,
    pub assignee_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub due_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub id: Uuid,
    pub task_id: Uuid,
    pub content: String,
    pub checked: bool,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskReminder {
    pub id: Uuid,
    pub task_id: Uuid,
    pub user_id: Uuid,
    pub remind_at: DateTime<Utc>,
    pub fired: bool,
    pub created_at: DateTime<Utc>,
}

// ── 16-02: Calendar Events ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: Uuid,
    pub server_id: Uuid,
    pub channel_id: Option<Uuid>,
    pub creator_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub all_day: bool,
    pub rrule: Option<String>,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarRsvp {
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

// ── 16-03: File Versioning ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    pub id: Uuid,
    pub attachment_id: Uuid,
    pub uploader_id: Uuid,
    pub version_number: i32,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i64,
    pub storage_key: String,
    pub sha256: Option<String>,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStorageQuota {
    pub server_id: Uuid,
    pub max_bytes: i64,
    pub used_bytes: i64,
    pub updated_at: DateTime<Utc>,
}

// ── 16-04: AI-Assist Preferences ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct AiPreferences {
    pub user_id: Uuid,
    pub summaries_enabled: bool,
    pub smart_replies: bool,
    pub auto_mod_suggest: bool,
    pub digest_enabled: bool,
    pub digest_interval: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDigest {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub summary: String,
    pub message_count: i32,
    pub created_at: DateTime<Utc>,
}
