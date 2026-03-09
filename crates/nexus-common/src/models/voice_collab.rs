use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── 22-01: Video Grid Polish ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoLayout {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub layout_type: String,
    pub pinned_users: serde_json::Value,
    pub custom_positions: serde_json::Value,
    pub pip_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualBackground {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub bg_type: String,
    pub image_url: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

// ── 22-02: Live Streaming ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveStream {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub streamer_id: Uuid,
    pub title: String,
    pub status: String,
    pub viewer_count: i32,
    pub max_viewers: i32,
    pub is_e2ee: bool,
    pub recording_url: Option<String>,
    pub hls_url: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamViewer {
    pub stream_id: Uuid,
    pub user_id: Uuid,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
}

// ── 22-03: Collaborative Sessions ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakoutRoom {
    pub id: Uuid,
    pub parent_channel: Uuid,
    pub name: String,
    pub capacity: i32,
    pub status: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabSession {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub session_type: String,
    pub document_id: Option<Uuid>,
    pub participants: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

// ── 22-04: Spatial & Immersive Audio ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialAudioConfig {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub preset: String,
    pub room_width: f32,
    pub room_depth: f32,
    pub room_height: f32,
    pub positions: serde_json::Value,
    pub hrtf_enabled: bool,
    pub ambisonics_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── 22-05: Low-Latency & Quality ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoicePreset {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_latency_ms: i32,
    pub jitter_buffer_ms: i32,
    pub fec_level: i32,
    pub dtx_enabled: bool,
    pub normalization: bool,
    pub is_builtin: bool,
    pub created_at: DateTime<Utc>,
}
