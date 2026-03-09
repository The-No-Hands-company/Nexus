use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── 21-01: Semantic Search ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEmbedding {
    pub id: Uuid,
    pub message_id: Uuid,
    pub channel_id: Uuid,
    pub embedding: Option<Vec<u8>>,
    pub model_name: String,
    pub model_version: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub id: Uuid,
    pub user_id: Uuid,
    pub raw_query: String,
    pub parsed_filters: serde_json::Value,
    pub result_count: i32,
    pub latency_ms: i32,
    pub created_at: DateTime<Utc>,
}

// ── 21-02: Proactive Assists ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSuggestion {
    pub id: Uuid,
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub suggestion_type: String,
    pub content: String,
    pub context_ids: serde_json::Value,
    pub model_name: String,
    pub accepted: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSummary {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub channel_id: Uuid,
    pub summary: String,
    pub message_count: i32,
    pub model_name: String,
    pub model_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── 21-03: Moderation AI ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToxicityScore {
    pub id: Uuid,
    pub message_id: Uuid,
    pub server_id: Uuid,
    pub score: f32,
    pub categories: serde_json::Value,
    pub model_name: String,
    pub flagged: bool,
    pub reviewed: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidDetection {
    pub id: Uuid,
    pub server_id: Uuid,
    pub detection_type: String,
    pub severity: String,
    pub details: serde_json::Value,
    pub auto_actions: serde_json::Value,
    pub resolved: bool,
    pub detected_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

// ── 21-04: Voice AI ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceTranscript {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub session_id: Option<Uuid>,
    pub speaker_id: Option<Uuid>,
    pub segment_start: f32,
    pub segment_end: f32,
    pub text: String,
    pub language: String,
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommand {
    pub id: Uuid,
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub command_text: String,
    pub action: String,
    pub confidence: f32,
    pub executed: bool,
    pub created_at: DateTime<Utc>,
}

// ── 21-05: AI Governance ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConsent {
    pub user_id: Uuid,
    pub server_id: Uuid,
    pub feature: String,
    pub enabled: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAuditEntry {
    pub id: Uuid,
    pub server_id: Uuid,
    pub feature: String,
    pub action: String,
    pub actor_id: Option<Uuid>,
    pub details: serde_json::Value,
    pub model_name: Option<String>,
    pub model_version: Option<String>,
    pub created_at: DateTime<Utc>,
}
