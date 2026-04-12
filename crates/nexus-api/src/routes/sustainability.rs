//! Sustainability & extensibility routes — Phase 24.
//!
//! Protocol versions, capability negotiations, governance polls & proposals,
//! contributor badges, security audits, tutorial progress, migration guides.

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::models::sustainability::*;
use nexus_db::repository::{sustainability, members};
use nexus_common::models::Member;
use serde::Deserialize;
use sqlx;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// ── Router ─────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Protocol versions
        .route("/admin/protocol-versions", get(list_protocol_versions).post(upsert_protocol_version))
        // Governance polls
        .route(
            "/servers/{server_id}/governance/polls",
            get(list_governance_polls).post(create_governance_poll),
        )
        .route(
            "/servers/{server_id}/governance/polls/{poll_id}/vote",
            post(cast_vote),
        )
        // Governance proposals
        .route(
            "/servers/{server_id}/governance/proposals",
            get(list_governance_proposals).post(create_governance_proposal),
        )
        // Contributor badges
        .route(
            "/users/{user_id}/contributor-badges",
            get(list_contributor_badges),
        )
        .route(
            "/users/{user_id}/contributor-badges/award",
            post(award_contributor_badge),
        )
        // Security audits (admin)
        .route("/admin/security-audits", get(list_security_audits).post(create_security_audit))
        .route("/admin/vulnerability-records", get(list_vulnerability_records_handler).post(create_vulnerability_record))
        // Tutorial progress
        .route(
            "/users/@me/tutorial-progress",
            get(list_tutorial_progress).post(upsert_tutorial_progress),
        )
        // Migration guides
        .route("/migration-guides", get(list_migration_guides))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request / Query ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LimitQuery { limit: Option<i64> }

#[derive(Debug, Deserialize)]
struct ProtocolQuery { protocol: String }

#[derive(Debug, Deserialize)]
struct UpsertProtocolVersionReq {
    protocol: String,
    version: String,
    status: Option<String>,
    capabilities: Option<serde_json::Value>,
    migration_guide: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateGovernancePollReq {
    title: String,
    description: Option<String>,
    poll_type: Option<String>,
    options: serde_json::Value,
    min_participation: Option<f32>,
    allow_multiple: Option<bool>,
    anonymous: Option<bool>,
    closes_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CastVoteReq {
    option_index: i32,
}

#[derive(Debug, Deserialize)]
struct CreateGovernanceProposalReq {
    title: String,
    body: String,
    discussion_channel: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct AwardContributorBadgeReq {
    badge_type: String,
    source: Option<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct CreateSecurityAuditReq {
    audit_type: String,
    auditor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateVulnerabilityReq {
    audit_id: Option<Uuid>,
    cve_id: Option<String>,
    package_name: String,
    severity: String,
    description: String,
    remediation: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpsertTutorialProgressReq {
    tutorial_id: String,
    completed_steps: serde_json::Value,
    completed: Option<bool>,
}

// ── Handlers ──────────────────────────────────────────────────────────────

async fn list_protocol_versions(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Query(q): Query<ProtocolQuery>,
) -> NexusResult<Json<Vec<ProtocolVersion>>> {
    let rows = sustainability::list_protocol_versions(&state.db.pool, &q.protocol)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn upsert_protocol_version(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Json(body): Json<UpsertProtocolVersionReq>,
) -> NexusResult<Json<ProtocolVersion>> {
    let row = sustainability::upsert_protocol_version(
        &state.db.pool, Uuid::new_v4(), &body.protocol, &body.version,
        &body.status.unwrap_or_else(|| "stable".into()),
        &body.capabilities.unwrap_or(serde_json::json!({})),
        body.migration_guide.as_deref(),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_governance_polls(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<GovernancePoll>>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let rows = sustainability::list_governance_polls(&state.db.pool, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_governance_poll(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateGovernancePollReq>,
) -> NexusResult<Json<GovernancePoll>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let row = sustainability::create_governance_poll(
        &state.db.pool, Uuid::new_v4(), server_id, &body.title,
        body.description.as_deref(),
        &body.poll_type.unwrap_or_else(|| "simple".into()),
        &body.options, body.min_participation.unwrap_or(0.0),
        body.allow_multiple.unwrap_or(false),
        body.anonymous.unwrap_or(false),
        ctx.user_id, body.closes_at.as_deref(),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn cast_vote(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path((server_id, poll_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CastVoteReq>,
) -> NexusResult<Json<PollVote>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let row = sustainability::cast_vote(&state.db.pool, poll_id, ctx.user_id, body.option_index)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_governance_proposals(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<GovernanceProposal>>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let rows = sustainability::list_governance_proposals(&state.db.pool, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_governance_proposal(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateGovernanceProposalReq>,
) -> NexusResult<Json<GovernanceProposal>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() { return Err(NexusError::Forbidden); }

    let row = sustainability::create_governance_proposal(
        &state.db.pool, Uuid::new_v4(), server_id, &body.title,
        &body.body, ctx.user_id, body.discussion_channel,
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_contributor_badges(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(user_id): Path<Uuid>,
) -> NexusResult<Json<Vec<ContributorBadge>>> {
    let rows = sustainability::list_contributor_badges(&state.db.pool, user_id)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn award_contributor_badge(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<AwardContributorBadgeReq>,
) -> NexusResult<Json<ContributorBadge>> {
    let row = sustainability::award_contributor_badge(
        &state.db.pool, Uuid::new_v4(), user_id, &body.badge_type,
        body.source.as_deref(),
        &body.metadata.unwrap_or(serde_json::json!({})),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_security_audits(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<SecurityAudit>>> {
    let rows = sustainability::list_security_audits(&state.db.pool, q.limit.unwrap_or(50))
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_security_audit(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Json(body): Json<CreateSecurityAuditReq>,
) -> NexusResult<Json<SecurityAudit>> {
    let row = sustainability::create_security_audit(
        &state.db.pool, Uuid::new_v4(), &body.audit_type, body.auditor.as_deref(),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_vulnerability_records_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<VulnerabilityRecord>>> {
    // Reuse security audits query to get records (custom query needed for full listing)
    let q_str = format!(
        "SELECT {} FROM vulnerability_records ORDER BY created_at DESC LIMIT $1",
        nexus_db::select_cols::VULNERABILITY_RECORD_COLS
    );
    let rows = sqlx::query_as::<_, VulnerabilityRecord>(&q_str)
        .bind(q.limit.unwrap_or(100))
        .fetch_all(&state.db.pool)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_vulnerability_record(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Json(body): Json<CreateVulnerabilityReq>,
) -> NexusResult<Json<VulnerabilityRecord>> {
    let row = sustainability::create_vulnerability_record(
        &state.db.pool, Uuid::new_v4(), body.audit_id,
        body.cve_id.as_deref(), &body.package_name,
        &body.severity, &body.description, body.remediation.as_deref(),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_tutorial_progress(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<TutorialProgress>>> {
    let q_str = format!(
        "SELECT {} FROM tutorial_progress WHERE user_id = $1 ORDER BY started_at DESC",
        nexus_db::select_cols::TUTORIAL_PROGRESS_COLS
    );
    let rows = sqlx::query_as::<_, TutorialProgress>(&q_str)
        .bind(ctx.user_id.to_string())
        .fetch_all(&state.db.pool)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn upsert_tutorial_progress(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<UpsertTutorialProgressReq>,
) -> NexusResult<Json<TutorialProgress>> {
    let row = sustainability::upsert_tutorial_progress(
        &state.db.pool, ctx.user_id, &body.tutorial_id,
        &body.completed_steps, body.completed.unwrap_or(false),
    ).await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_migration_guides(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<MigrationGuide>>> {
    let rows = sustainability::list_migration_guides(&state.db.pool)
        .await.map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}
