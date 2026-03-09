use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── 24-01: Protocol Evolution ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub id: Uuid,
    pub protocol: String,
    pub version: String,
    pub status: String,
    pub capabilities: serde_json::Value,
    pub deprecation_date: Option<DateTime<Utc>>,
    pub sunset_date: Option<DateTime<Utc>>,
    pub migration_guide: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityNegotiation {
    pub id: Uuid,
    pub local_instance: String,
    pub remote_instance: String,
    pub local_caps: serde_json::Value,
    pub remote_caps: serde_json::Value,
    pub agreed_caps: serde_json::Value,
    pub protocol_version: String,
    pub negotiated_at: DateTime<Utc>,
}

// ── 24-02: Community Governance ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernancePoll {
    pub id: Uuid,
    pub server_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub poll_type: String,
    pub options: serde_json::Value,
    pub min_participation: f32,
    pub allow_multiple: bool,
    pub anonymous: bool,
    pub status: String,
    pub created_by: Uuid,
    pub opens_at: DateTime<Utc>,
    pub closes_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollVote {
    pub poll_id: Uuid,
    pub user_id: Uuid,
    pub option_index: i32,
    pub voted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceProposal {
    pub id: Uuid,
    pub server_id: Uuid,
    pub title: String,
    pub body: String,
    pub status: String,
    pub author_id: Uuid,
    pub discussion_channel: Option<Uuid>,
    pub poll_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorBadge {
    pub id: Uuid,
    pub user_id: Uuid,
    pub badge_type: String,
    pub source: Option<String>,
    pub verified: bool,
    pub metadata: serde_json::Value,
    pub awarded_at: DateTime<Utc>,
}

// ── 24-03: Security & Threat Modeling ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAudit {
    pub id: Uuid,
    pub audit_type: String,
    pub status: String,
    pub findings: serde_json::Value,
    pub severity_summary: serde_json::Value,
    pub auditor: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityRecord {
    pub id: Uuid,
    pub audit_id: Option<Uuid>,
    pub cve_id: Option<String>,
    pub package_name: String,
    pub severity: String,
    pub description: String,
    pub remediation: Option<String>,
    pub status: String,
    pub discovered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

// ── 24-04: Documentation & Education ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TutorialProgress {
    pub user_id: Uuid,
    pub tutorial_id: String,
    pub completed_steps: serde_json::Value,
    pub completed: bool,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationGuide {
    pub id: Uuid,
    pub from_platform: String,
    pub title: String,
    pub content: String,
    pub version: String,
    pub is_published: bool,
    pub author_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
