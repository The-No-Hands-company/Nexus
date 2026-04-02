use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── 19-01: Data Portability & Migration ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportJob {
    pub id: Uuid,
    pub server_id: Uuid,
    pub user_id: Uuid,
    /// Canonical source kind used by the import pipeline.
    ///
    /// Stored in the legacy DB column name `source_platform` for compatibility.
    pub source_platform: String,
    pub status: String,
    pub total_items: i32,
    pub imported_items: i32,
    pub error_log: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkInvitation {
    pub id: Uuid,
    pub server_id: Uuid,
    pub inviter_id: Uuid,
    pub emails: serde_json::Value,
    pub status: String,
    pub sent_count: i32,
    pub total_count: i32,
    pub invite_code: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ── 19-02: Onboarding ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub icon_url: Option<String>,
    pub channels: serde_json::Value,
    pub roles: serde_json::Value,
    pub settings: serde_json::Value,
    pub is_builtin: bool,
    pub creator_id: Option<Uuid>,
    pub usage_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingProgress {
    pub user_id: Uuid,
    pub completed_steps: serde_json::Value,
    pub dismissed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── 19-03: Analytics ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerAnalyticsSnapshot {
    pub id: Uuid,
    pub server_id: Uuid,
    pub period_date: String,
    pub messages_count: i32,
    pub active_members: i32,
    pub new_members: i32,
    pub left_members: i32,
    pub voice_minutes: i32,
    pub reports_resolved: i32,
    pub bans_issued: i32,
    pub filters_triggered: i32,
    pub created_at: DateTime<Utc>,
}

// ── 19-04: Plugin Marketplace ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplacePlugin {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub author_id: Option<Uuid>,
    pub version: String,
    pub manifest_url: String,
    pub icon_url: Option<String>,
    pub source_url: Option<String>,
    pub signature: Option<String>,
    pub signing_key_id: Option<String>,
    pub category: String,
    pub tags: serde_json::Value,
    pub downloads: i64,
    pub avg_rating: f32,
    pub rating_count: i32,
    pub is_verified: bool,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginReview {
    pub id: Uuid,
    pub plugin_id: Uuid,
    pub user_id: Uuid,
    pub rating: i16,
    pub title: Option<String>,
    pub body: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstall {
    pub id: Uuid,
    pub plugin_id: Uuid,
    pub server_id: Uuid,
    pub installed_by: Uuid,
    pub version: String,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
}
