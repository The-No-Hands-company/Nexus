//! Repository: sustainability & extensibility — Phase 24.

use nexus_common::models::sustainability::*;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::*;

// ── 24-01: Protocol Versions ──────────────────────────────────────────────

pub async fn upsert_protocol_version(
    pool: &AnyPool,
    id: Uuid,
    protocol: &str,
    version: &str,
    status: &str,
    capabilities: &serde_json::Value,
    migration_guide: Option<&str>,
) -> Result<ProtocolVersion, sqlx::Error> {
    let q = format!(
        "INSERT INTO protocol_versions (id, protocol, version, status, capabilities, migration_guide) \
         VALUES ($1, $2, $3, $4, $5::jsonb, $6) \
         ON CONFLICT (protocol, version) DO UPDATE SET \
           status = $4, capabilities = $5::jsonb, migration_guide = $6 \
         RETURNING {PROTOCOL_VERSION_COLS}"
    );
    sqlx::query_as::<_, ProtocolVersion>(&q)
        .bind(id.to_string())
        .bind(protocol)
        .bind(version)
        .bind(status)
        .bind(capabilities.to_string())
        .bind(migration_guide)
        .fetch_one(pool)
        .await
}

pub async fn list_protocol_versions(
    pool: &AnyPool,
    protocol: &str,
) -> Result<Vec<ProtocolVersion>, sqlx::Error> {
    let q = format!(
        "SELECT {PROTOCOL_VERSION_COLS} FROM protocol_versions WHERE protocol = $1 ORDER BY created_at DESC"
    );
    sqlx::query_as::<_, ProtocolVersion>(&q)
        .bind(protocol)
        .fetch_all(pool)
        .await
}

pub async fn upsert_capability_negotiation(
    pool: &AnyPool,
    id: Uuid,
    local_instance: &str,
    remote_instance: &str,
    local_caps: &serde_json::Value,
    remote_caps: &serde_json::Value,
    agreed_caps: &serde_json::Value,
    protocol_version: &str,
) -> Result<CapabilityNegotiation, sqlx::Error> {
    let q = format!(
        "INSERT INTO capability_negotiations \
         (id, local_instance, remote_instance, local_caps, remote_caps, agreed_caps, protocol_version) \
         VALUES ($1, $2, $3, $4::jsonb, $5::jsonb, $6::jsonb, $7) \
         ON CONFLICT (local_instance, remote_instance) DO UPDATE SET \
           local_caps = $4::jsonb, remote_caps = $5::jsonb, agreed_caps = $6::jsonb, \
           protocol_version = $7, negotiated_at = now() \
         RETURNING {CAPABILITY_NEGOTIATION_COLS}"
    );
    sqlx::query_as::<_, CapabilityNegotiation>(&q)
        .bind(id.to_string())
        .bind(local_instance)
        .bind(remote_instance)
        .bind(local_caps.to_string())
        .bind(remote_caps.to_string())
        .bind(agreed_caps.to_string())
        .bind(protocol_version)
        .fetch_one(pool)
        .await
}

// ── 24-02: Governance ─────────────────────────────────────────────────────

pub async fn create_governance_poll(
    pool: &AnyPool,
    id: Uuid,
    server_id: Uuid,
    title: &str,
    description: Option<&str>,
    poll_type: &str,
    options: &serde_json::Value,
    min_participation: f32,
    allow_multiple: bool,
    anonymous: bool,
    created_by: Uuid,
    closes_at: Option<&str>,
) -> Result<GovernancePoll, sqlx::Error> {
    let q = format!(
        "INSERT INTO governance_polls \
         (id, server_id, title, description, poll_type, options, min_participation, \
          allow_multiple, anonymous, created_by, closes_at) \
         VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8, $9, $10, $11::timestamptz) \
         RETURNING {GOVERNANCE_POLL_COLS}"
    );
    sqlx::query_as::<_, GovernancePoll>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(title)
        .bind(description)
        .bind(poll_type)
        .bind(options.to_string())
        .bind(min_participation as f64)
        .bind(allow_multiple)
        .bind(anonymous)
        .bind(created_by.to_string())
        .bind(closes_at)
        .fetch_one(pool)
        .await
}

pub async fn list_governance_polls(
    pool: &AnyPool,
    server_id: Uuid,
) -> Result<Vec<GovernancePoll>, sqlx::Error> {
    let q = format!(
        "SELECT {GOVERNANCE_POLL_COLS} FROM governance_polls WHERE server_id = $1 ORDER BY created_at DESC"
    );
    sqlx::query_as::<_, GovernancePoll>(&q)
        .bind(server_id.to_string())
        .fetch_all(pool)
        .await
}

pub async fn cast_vote(
    pool: &AnyPool,
    poll_id: Uuid,
    user_id: Uuid,
    option_index: i32,
) -> Result<PollVote, sqlx::Error> {
    let q = format!(
        "INSERT INTO poll_votes (poll_id, user_id, option_index) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (poll_id, user_id, option_index) DO UPDATE SET voted_at = poll_votes.voted_at \
         RETURNING {POLL_VOTE_COLS}"
    );
    sqlx::query_as::<_, PollVote>(&q)
        .bind(poll_id.to_string())
        .bind(user_id.to_string())
        .bind(option_index)
        .fetch_one(pool)
        .await
}

pub async fn create_governance_proposal(
    pool: &AnyPool,
    id: Uuid,
    server_id: Uuid,
    title: &str,
    body: &str,
    author_id: Uuid,
    discussion_channel: Option<Uuid>,
) -> Result<GovernanceProposal, sqlx::Error> {
    let q = format!(
        "INSERT INTO governance_proposals (id, server_id, title, body, author_id, discussion_channel) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING {GOVERNANCE_PROPOSAL_COLS}"
    );
    sqlx::query_as::<_, GovernanceProposal>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(title)
        .bind(body)
        .bind(author_id.to_string())
        .bind(discussion_channel.map(|u| u.to_string()))
        .fetch_one(pool)
        .await
}

pub async fn list_governance_proposals(
    pool: &AnyPool,
    server_id: Uuid,
) -> Result<Vec<GovernanceProposal>, sqlx::Error> {
    let q = format!(
        "SELECT {GOVERNANCE_PROPOSAL_COLS} FROM governance_proposals WHERE server_id = $1 ORDER BY created_at DESC"
    );
    sqlx::query_as::<_, GovernanceProposal>(&q)
        .bind(server_id.to_string())
        .fetch_all(pool)
        .await
}

pub async fn award_contributor_badge(
    pool: &AnyPool,
    id: Uuid,
    user_id: Uuid,
    badge_type: &str,
    source: Option<&str>,
    metadata: &serde_json::Value,
) -> Result<ContributorBadge, sqlx::Error> {
    let q = format!(
        "INSERT INTO contributor_badges (id, user_id, badge_type, source, metadata) \
         VALUES ($1, $2, $3, $4, $5::jsonb) \
         ON CONFLICT (user_id, badge_type) DO UPDATE SET source = $4, metadata = $5::jsonb \
         RETURNING {CONTRIBUTOR_BADGE_COLS}"
    );
    sqlx::query_as::<_, ContributorBadge>(&q)
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(badge_type)
        .bind(source)
        .bind(metadata.to_string())
        .fetch_one(pool)
        .await
}

pub async fn list_contributor_badges(
    pool: &AnyPool,
    user_id: Uuid,
) -> Result<Vec<ContributorBadge>, sqlx::Error> {
    let q = format!(
        "SELECT {CONTRIBUTOR_BADGE_COLS} FROM contributor_badges WHERE user_id = $1 ORDER BY awarded_at DESC"
    );
    sqlx::query_as::<_, ContributorBadge>(&q)
        .bind(user_id.to_string())
        .fetch_all(pool)
        .await
}

// ── 24-03: Security Audits ────────────────────────────────────────────────

pub async fn create_security_audit(
    pool: &AnyPool,
    id: Uuid,
    audit_type: &str,
    auditor: Option<&str>,
) -> Result<SecurityAudit, sqlx::Error> {
    let q = format!(
        "INSERT INTO security_audits (id, audit_type, auditor) \
         VALUES ($1, $2, $3) \
         RETURNING {SECURITY_AUDIT_COLS}"
    );
    sqlx::query_as::<_, SecurityAudit>(&q)
        .bind(id.to_string())
        .bind(audit_type)
        .bind(auditor)
        .fetch_one(pool)
        .await
}

pub async fn list_security_audits(
    pool: &AnyPool,
    limit: i64,
) -> Result<Vec<SecurityAudit>, sqlx::Error> {
    let q = format!(
        "SELECT {SECURITY_AUDIT_COLS} FROM security_audits ORDER BY created_at DESC LIMIT $1"
    );
    sqlx::query_as::<_, SecurityAudit>(&q)
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn create_vulnerability_record(
    pool: &AnyPool,
    id: Uuid,
    audit_id: Option<Uuid>,
    cve_id: Option<&str>,
    package_name: &str,
    severity: &str,
    description: &str,
    remediation: Option<&str>,
) -> Result<VulnerabilityRecord, sqlx::Error> {
    let q = format!(
        "INSERT INTO vulnerability_records \
         (id, audit_id, cve_id, package_name, severity, description, remediation) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING {VULNERABILITY_RECORD_COLS}"
    );
    sqlx::query_as::<_, VulnerabilityRecord>(&q)
        .bind(id.to_string())
        .bind(audit_id.map(|u| u.to_string()))
        .bind(cve_id)
        .bind(package_name)
        .bind(severity)
        .bind(description)
        .bind(remediation)
        .fetch_one(pool)
        .await
}

// ── 24-04: Tutorials & Migration Guides ───────────────────────────────────

pub async fn upsert_tutorial_progress(
    pool: &AnyPool,
    user_id: Uuid,
    tutorial_id: &str,
    completed_steps: &serde_json::Value,
    completed: bool,
) -> Result<TutorialProgress, sqlx::Error> {
    let q = format!(
        "INSERT INTO tutorial_progress (user_id, tutorial_id, completed_steps, completed) \
         VALUES ($1, $2, $3::jsonb, $4) \
         ON CONFLICT (user_id, tutorial_id) DO UPDATE SET \
           completed_steps = $3::jsonb, completed = $4, \
           completed_at = CASE WHEN $4 THEN now() ELSE NULL END \
         RETURNING {TUTORIAL_PROGRESS_COLS}"
    );
    sqlx::query_as::<_, TutorialProgress>(&q)
        .bind(user_id.to_string())
        .bind(tutorial_id)
        .bind(completed_steps.to_string())
        .bind(completed)
        .fetch_one(pool)
        .await
}

pub async fn upsert_migration_guide(
    pool: &AnyPool,
    id: Uuid,
    from_platform: &str,
    title: &str,
    content: &str,
    version: &str,
    is_published: bool,
    author_id: Option<Uuid>,
) -> Result<MigrationGuide, sqlx::Error> {
    let q = format!(
        "INSERT INTO migration_guides (id, from_platform, title, content, version, is_published, author_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (from_platform, version) DO UPDATE SET \
           title = $3, content = $4, is_published = $6, author_id = $7, updated_at = now() \
         RETURNING {MIGRATION_GUIDE_COLS}"
    );
    sqlx::query_as::<_, MigrationGuide>(&q)
        .bind(id.to_string())
        .bind(from_platform)
        .bind(title)
        .bind(content)
        .bind(version)
        .bind(is_published)
        .bind(author_id.map(|u| u.to_string()))
        .fetch_one(pool)
        .await
}

pub async fn list_migration_guides(pool: &AnyPool) -> Result<Vec<MigrationGuide>, sqlx::Error> {
    let q = format!(
        "SELECT {MIGRATION_GUIDE_COLS} FROM migration_guides WHERE is_published = TRUE ORDER BY from_platform"
    );
    sqlx::query_as::<_, MigrationGuide>(&q)
        .fetch_all(pool)
        .await
}
