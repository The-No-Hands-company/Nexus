//! Phase 18 — Accessibility & Inclusivity models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Per-user accessibility preferences (server-synced).
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccessibilitySettings {
    pub user_id: Uuid,

    // Screen reader
    pub screen_reader_mode: bool,
    pub announce_messages: bool,
    pub announce_reactions: bool,
    pub announce_typing: bool,
    pub keyboard_shortcuts: bool,

    // Visual
    pub high_contrast_mode: bool,
    pub reduced_motion: bool,
    pub font_family: String,
    pub custom_font_name: Option<String>,
    pub color_blind_mode: String,

    // Language
    pub preferred_language: String,
    pub auto_translate: bool,
    pub rtl_override: bool,

    // Voice accessibility
    pub captions_enabled: bool,
    pub caption_font_size: String,
    pub caption_position: String,
    pub tts_enabled: bool,
    pub tts_rate: f32,
    pub tts_voice: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A live caption segment from voice channel speech recognition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCaption {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub speaker_id: Uuid,
    pub text: String,
    pub language: String,
    pub is_final: bool,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// A request to read a message aloud via TTS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTtsRequest {
    pub id: Uuid,
    pub user_id: Uuid,
    pub message_id: Uuid,
    pub channel_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Cached translation for a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTranslation {
    pub id: Uuid,
    pub message_id: Uuid,
    pub source_language: String,
    pub target_language: String,
    pub translated_text: String,
    pub created_at: DateTime<Utc>,
}
