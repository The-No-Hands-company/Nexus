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

// ── 20-01: Store Governance & Trust Model ────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Draft,
    Submitted,
    Scanning,
    Review,
    Approved,
    Rejected,
    Quarantined,
    Takedown,
    Archived,
}

impl std::fmt::Display for ReviewStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewStatus::Draft => write!(f, "draft"),
            ReviewStatus::Submitted => write!(f, "submitted"),
            ReviewStatus::Scanning => write!(f, "scanning"),
            ReviewStatus::Review => write!(f, "review"),
            ReviewStatus::Approved => write!(f, "approved"),
            ReviewStatus::Rejected => write!(f, "rejected"),
            ReviewStatus::Quarantined => write!(f, "quarantined"),
            ReviewStatus::Takedown => write!(f, "takedown"),
            ReviewStatus::Archived => write!(f, "archived"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustTier {
    Unlisted,
    Reviewed,
    Verified,
}

impl std::fmt::Display for TrustTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustTier::Unlisted => write!(f, "unlisted"),
            TrustTier::Reviewed => write!(f, "reviewed"),
            TrustTier::Verified => write!(f, "verified"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityLevel {
    Unverified,
    EmailVerified,
    DomainVerified,
    LegalVerified,
}

impl std::fmt::Display for IdentityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentityLevel::Unverified => write!(f, "unverified"),
            IdentityLevel::EmailVerified => write!(f, "email_verified"),
            IdentityLevel::DomainVerified => write!(f, "domain_verified"),
            IdentityLevel::LegalVerified => write!(f, "legal_verified"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplacePluginGovernance {
    pub id: Uuid,
    pub review_status: ReviewStatus,
    pub trust_tier: TrustTier,
    pub creator_identity_level: IdentityLevel,
    pub provenance_verified: bool,
    pub rights_attestation: Option<String>,
    pub security_scan_result: Option<serde_json::Value>,
    pub last_scanned_at: Option<DateTime<Utc>>,
    pub rejected_reason: Option<String>,
    pub quarantine_reason: Option<String>,
    pub review_notes: Option<String>,
    pub reviewer_id: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub takedown_requested_by: Option<Uuid>,
    pub takedown_reason: Option<String>,
    pub takedown_requested_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceReview {
    pub id: Uuid,
    pub plugin_id: Uuid,
    pub reviewer_id: Option<Uuid>,
    pub previous_status: ReviewStatus,
    pub new_status: ReviewStatus,
    pub action: String,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorVetting {
    pub id: Uuid,
    pub user_id: Uuid,
    pub identity_level: IdentityLevel,
    pub identity_documents: Option<serde_json::Value>, // Encrypted in DB
    pub domain: Option<String>,
    pub domain_verified: bool,
    pub legal_name: Option<String>, // Encrypted in DB
    pub legal_entity_id: Option<String>, // Encrypted in DB
    pub rights_attestation: Option<String>,
    pub two_factor_enabled: bool,
    pub ip_whitelist: Option<serde_json::Value>,
    pub status: String,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceMonetization {
    pub id: Uuid,
    pub plugin_id: Uuid,
    pub creator_id: Uuid,
    pub is_monetized: bool,
    pub price_cents: Option<i32>,
    pub currency: String,
    pub revenue_share_pct: f32,
    pub total_sales_cents: i64,
    pub creator_earnings_cents: i64,
    pub platform_earnings_cents: i64,
    pub payout_address: Option<String>, // Encrypted in DB
    pub last_payout_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceTakedown {
    pub id: Uuid,
    pub plugin_id: Uuid,
    pub reported_by: Option<Uuid>,
    pub reason: String,
    pub description: String,
    pub evidence_urls: Option<serde_json::Value>,
    pub quarantine_status: String,
    pub reviewer_id: Option<Uuid>,
    pub review_notes: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reinstated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
