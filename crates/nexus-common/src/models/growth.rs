use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── 23-01: Onboarding Personalization ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerRecommendation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub server_id: Uuid,
    pub score: f32,
    pub reason: String,
    pub dismissed: bool,
    pub joined: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingFlow {
    pub id: Uuid,
    pub server_id: Uuid,
    pub steps: serde_json::Value,
    pub adaptive: bool,
    pub skip_completed: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── 23-02: Cross-Device Continuity ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: String,
    pub device_type: String,
    pub last_channel_id: Option<Uuid>,
    pub scroll_position: Option<serde_json::Value>,
    pub is_active: bool,
    pub last_seen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardSync {
    pub id: Uuid,
    pub user_id: Uuid,
    pub source_device: String,
    pub content_type: String,
    pub encrypted_data: Vec<u8>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ── 23-03: Gamification Lite ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserXp {
    pub user_id: Uuid,
    pub server_id: Uuid,
    pub xp: i64,
    pub level: i32,
    pub last_xp_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamificationConfig {
    pub server_id: Uuid,
    pub enabled: bool,
    pub xp_per_message: i32,
    pub xp_per_reaction: i32,
    pub xp_per_voice_min: i32,
    pub level_formula: String,
    pub streak_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub criteria: serde_json::Value,
    pub reward_xp: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAchievement {
    pub user_id: Uuid,
    pub achievement_id: Uuid,
    pub earned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityStreak {
    pub user_id: Uuid,
    pub server_id: Uuid,
    pub current_streak: i32,
    pub longest_streak: i32,
    pub last_active_date: String,
}

// ── 23-04: Offline-First ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCursor {
    pub user_id: Uuid,
    pub device_id: String,
    pub channel_id: Uuid,
    pub last_message_id: Option<Uuid>,
    pub last_synced_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineQueueItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: String,
    pub action_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub conflict_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
}
