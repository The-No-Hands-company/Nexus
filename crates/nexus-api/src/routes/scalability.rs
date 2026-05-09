//! Scalability & performance hardening routes — Phase 20.
//!
//! Scaling configs, SFU nodes, federation batches, voice quality,
//! large-server tools, and operational metrics.

use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    middleware,
    routing::get,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::models::Member;
use nexus_common::models::scalability::*;
use nexus_db::repository::{
    federation_batches, large_server, members, ops_metrics, scaling_configs, sfu_nodes,
    voice_quality,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{AppState, middleware::AuthContext};

// ── Router ─────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Scaling configs
        .route(
            "/admin/scaling-configs",
            get(list_scaling_configs_handler).post(upsert_scaling_config_handler),
        )
        .route(
            "/admin/scaling-configs/{config_id}",
            get(get_scaling_config_handler),
        )
        // SFU nodes
        .route(
            "/admin/sfu-nodes",
            get(list_sfu_nodes_handler).post(upsert_sfu_node_handler),
        )
        // Federation batches
        .route(
            "/admin/federation-batches",
            get(list_federation_batches_handler),
        )
        .route(
            "/admin/federation-routes",
            get(list_federation_routes_handler),
        )
        // Voice quality
        .route(
            "/channels/{channel_id}/voice-quality-logs",
            get(list_voice_quality_logs_handler).post(create_voice_quality_log_handler),
        )
        // Large-server tools
        .route(
            "/servers/{server_id}/prune-rules",
            get(get_prune_rule_handler).post(upsert_prune_rule_handler),
        )
        .route(
            "/channels/{channel_id}/slow-mode-overrides",
            get(list_slow_mode_overrides_handler).post(upsert_slow_mode_override_handler),
        )
        // Ops metrics
        .route(
            "/admin/scaling-metrics",
            get(list_metrics_handler).post(record_metric_handler),
        )
        .route(
            "/admin/upgrade-records",
            get(list_upgrade_records_handler).post(create_upgrade_record_handler),
        )
        .route_layer(middleware::from_fn(
            crate::middleware::combined_auth_middleware,
        ))
}

// ── Request / Query ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LimitQuery {
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RegionQuery {
    region: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InstanceQuery {
    instance_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpsertScalingConfigReq {
    instance_id: String,
    region: String,
    shard_strategy: Option<String>,
    redis_mode: Option<String>,
    gateway_weight: Option<i32>,
    max_connections: Option<i32>,
    metadata: Option<serde_json::Value>,
    is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpsertSfuNodeReq {
    instance_id: String,
    region: String,
    hostname: String,
    port: Option<i32>,
    capacity: Option<i32>,
    status: Option<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct VoiceQualityLogReq {
    user_id: Uuid,
    sfu_node_id: Option<Uuid>,
    bitrate: Option<i32>,
    packet_loss: Option<f32>,
    jitter_ms: Option<f32>,
    latency_ms: Option<i32>,
    fec_enabled: Option<bool>,
    quality_score: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct UpsertPruneRuleReq {
    inactivity_days: Option<i32>,
    grace_period_days: Option<i32>,
    exclude_roles: Option<serde_json::Value>,
    notify_before: Option<bool>,
    is_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpsertSlowModeReq {
    role_id: Option<Uuid>,
    cooldown_secs: i32,
    escalation_mult: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct RecordMetricReq {
    instance_id: String,
    metric_name: String,
    metric_value: f32,
    unit: Option<String>,
    tags: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct CreateUpgradeRecordReq {
    from_version: String,
    to_version: String,
    notes: Option<String>,
    metadata: Option<serde_json::Value>,
}

// ── Handlers ──────────────────────────────────────────────────────────────

async fn list_scaling_configs_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<ScalingConfig>>> {
    let rows = scaling_configs::list_scaling_configs(&state.db.pool)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn upsert_scaling_config_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Json(body): Json<UpsertScalingConfigReq>,
) -> NexusResult<Json<ScalingConfig>> {
    let row = scaling_configs::upsert_scaling_config(
        &state.db.pool,
        Uuid::new_v4(),
        &body.instance_id,
        &body.region,
        &body
            .shard_strategy
            .unwrap_or_else(|| "consistent_hash".into()),
        &body.redis_mode.unwrap_or_else(|| "cluster".into()),
        body.gateway_weight.unwrap_or(1),
        body.max_connections.unwrap_or(10000),
        &body.metadata.unwrap_or(serde_json::json!({})),
        body.is_active.unwrap_or(true),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn get_scaling_config_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(config_id): Path<Uuid>,
) -> NexusResult<Json<ScalingConfig>> {
    let row = scaling_configs::get_scaling_config(&state.db.pool, config_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or(NexusError::NotFound {
            resource: "scaling_config".into(),
        })?;
    Ok(Json(row))
}

async fn list_sfu_nodes_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Query(q): Query<RegionQuery>,
) -> NexusResult<Json<Vec<SfuNode>>> {
    let rows = sfu_nodes::list_sfu_nodes(&state.db.pool, q.region.as_deref())
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn upsert_sfu_node_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Json(body): Json<UpsertSfuNodeReq>,
) -> NexusResult<Json<SfuNode>> {
    let row = sfu_nodes::upsert_sfu_node(
        &state.db.pool,
        Uuid::new_v4(),
        &body.instance_id,
        &body.region,
        &body.hostname,
        body.port.unwrap_or(443),
        body.capacity.unwrap_or(100),
        &body.status.unwrap_or_else(|| "active".into()),
        &body.metadata.unwrap_or(serde_json::json!({})),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_federation_batches_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<FederationEventBatch>>> {
    let rows = federation_batches::list_pending_batches(&state.db.pool, q.limit.unwrap_or(50))
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn list_federation_routes_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<FederationRoute>>> {
    let rows = federation_batches::list_federation_routes(&state.db.pool)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn list_voice_quality_logs_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Query(q): Query<LimitQuery>,
) -> NexusResult<Json<Vec<VoiceQualityLog>>> {
    let rows =
        voice_quality::list_voice_quality_logs(&state.db.pool, channel_id, q.limit.unwrap_or(100))
            .await
            .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_voice_quality_log_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<VoiceQualityLogReq>,
) -> NexusResult<Json<VoiceQualityLog>> {
    let row = voice_quality::create_voice_quality_log(
        &state.db.pool,
        Uuid::new_v4(),
        channel_id,
        body.user_id,
        body.sfu_node_id,
        body.bitrate.unwrap_or(64000),
        body.packet_loss.unwrap_or(0.0),
        body.jitter_ms.unwrap_or(0.0),
        body.latency_ms.unwrap_or(0),
        body.fec_enabled.unwrap_or(false),
        body.quality_score.unwrap_or(5.0),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn get_prune_rule_handler(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Option<MemberPruneRule>>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() {
        return Err(NexusError::Forbidden);
    }

    let row = large_server::get_prune_rule(&state.db.pool, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn upsert_prune_rule_handler(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<UpsertPruneRuleReq>,
) -> NexusResult<Json<MemberPruneRule>> {
    let _m: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _m.is_none() {
        return Err(NexusError::Forbidden);
    }

    let row = large_server::upsert_prune_rule(
        &state.db.pool,
        Uuid::new_v4(),
        server_id,
        body.inactivity_days.unwrap_or(30),
        body.grace_period_days.unwrap_or(7),
        &body.exclude_roles.unwrap_or(serde_json::json!([])),
        body.notify_before.unwrap_or(true),
        body.is_enabled.unwrap_or(true),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_slow_mode_overrides_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> NexusResult<Json<Vec<SlowModeOverride>>> {
    let rows = large_server::list_slow_mode_overrides(&state.db.pool, channel_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn upsert_slow_mode_override_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<UpsertSlowModeReq>,
) -> NexusResult<Json<SlowModeOverride>> {
    let row = large_server::upsert_slow_mode_override(
        &state.db.pool,
        Uuid::new_v4(),
        channel_id,
        body.role_id,
        body.cooldown_secs,
        body.escalation_mult.unwrap_or(1.0),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_metrics_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Query(q): Query<InstanceQuery>,
) -> NexusResult<Json<Vec<ScalingMetric>>> {
    let instance_id = q.instance_id.as_deref().unwrap_or("default");
    let rows = ops_metrics::list_metrics(&state.db.pool, instance_id, 200)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn record_metric_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Json(body): Json<RecordMetricReq>,
) -> NexusResult<Json<ScalingMetric>> {
    let row = ops_metrics::record_metric(
        &state.db.pool,
        Uuid::new_v4(),
        &body.instance_id,
        &body.metric_name,
        body.metric_value,
        &body.unit.unwrap_or_else(|| "count".into()),
        &body.tags.unwrap_or(serde_json::json!({})),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}

async fn list_upgrade_records_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<UpgradeRecord>>> {
    let rows = ops_metrics::list_upgrade_records(&state.db.pool)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(rows))
}

async fn create_upgrade_record_handler(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Json(body): Json<CreateUpgradeRecordReq>,
) -> NexusResult<Json<UpgradeRecord>> {
    let row = ops_metrics::create_upgrade_record(
        &state.db.pool,
        Uuid::new_v4(),
        &body.from_version,
        &body.to_version,
        body.notes.as_deref(),
        &body.metadata.unwrap_or(serde_json::json!({})),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(row))
}
