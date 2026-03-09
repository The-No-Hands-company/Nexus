//! Phase 17 — Multimedia & Expression Enhancement models.
//!
//! Voice/video notes, stories, drawings, voice settings, and media gallery.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── 17-01: Voice & Video Notes ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceNote {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub storage_key: String,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub duration_ms: i32,
    pub waveform: Option<String>,
    pub transcript: Option<String>,
    pub message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoNote {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub storage_key: String,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub duration_ms: i32,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub thumbnail_key: Option<String>,
    pub transcript: Option<String>,
    pub message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ── 17-02: Stories & Ephemeral Status ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    pub id: Uuid,
    pub author_id: Uuid,
    pub media_type: String,
    pub storage_key: Option<String>,
    pub text_content: Option<String>,
    pub text_style: Option<serde_json::Value>,
    pub expires_at: DateTime<Utc>,
    pub visibility: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryView {
    pub story_id: Uuid,
    pub viewer_id: Uuid,
    pub viewed_at: DateTime<Utc>,
}

// ── 17-03: In-Chat Drawing & Annotation ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drawing {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub drawing_data: serde_json::Value,
    pub width: i32,
    pub height: i32,
    pub preview_key: Option<String>,
    pub message_id: Option<Uuid>,
    pub is_whiteboard: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── 17-04: Advanced Voice Features ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSettings {
    pub user_id: Uuid,
    pub spatial_audio: bool,
    pub noise_gate_enabled: bool,
    pub noise_gate_threshold: f32,
    pub echo_cancel_level: String,
    pub auto_gain: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceMusicQueueItem {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub added_by: Uuid,
    pub title: String,
    pub source_url: String,
    pub duration_ms: i32,
    pub position: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

// ── 17-05: Media Gallery ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaGalleryFilter {
    pub id: Uuid,
    pub user_id: Uuid,
    pub server_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub name: String,
    pub media_types: Vec<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
