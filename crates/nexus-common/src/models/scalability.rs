use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── 20-01: Horizontal Scaling ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingConfig {
    pub id: Uuid,
    pub instance_id: String,
    pub region: String,
    pub shard_strategy: String,
    pub redis_mode: String,
    pub gateway_weight: i32,
    pub max_connections: i32,
    pub metadata: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SfuNode {
    pub id: Uuid,
    pub instance_id: String,
    pub region: String,
    pub hostname: String,
    pub port: i32,
    pub capacity: i32,
    pub current_load: i32,
    pub status: String,
    pub metadata: serde_json::Value,
    pub last_heartbeat: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ── 20-02: Federation Performance ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationEventBatch {
    pub id: Uuid,
    pub target_instance: String,
    pub events: serde_json::Value,
    pub event_count: i32,
    pub status: String,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationRoute {
    pub id: Uuid,
    pub source_instance: String,
    pub target_instance: String,
    pub latency_ms: i32,
    pub is_websocket: bool,
    pub priority: i32,
    pub status: String,
    pub last_probed: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationDedupEntry {
    pub event_id: String,
    pub source_instance: String,
    pub received_at: DateTime<Utc>,
}

// ── 20-03: Voice Network Resilience ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceQualityLog {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub sfu_node_id: Option<Uuid>,
    pub bitrate: i32,
    pub packet_loss: f32,
    pub jitter_ms: f32,
    pub latency_ms: i32,
    pub fec_enabled: bool,
    pub quality_score: f32,
    pub created_at: DateTime<Utc>,
}

// ── 20-04: Large-Server Tooling ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberPruneRule {
    pub id: Uuid,
    pub server_id: Uuid,
    pub inactivity_days: i32,
    pub grace_period_days: i32,
    pub exclude_roles: serde_json::Value,
    pub notify_before: bool,
    pub is_enabled: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub pruned_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowModeOverride {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub role_id: Option<Uuid>,
    pub cooldown_secs: i32,
    pub escalation_mult: f32,
    pub created_at: DateTime<Utc>,
}

// ── 20-05: Operational Excellence ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingMetric {
    pub id: Uuid,
    pub instance_id: String,
    pub metric_name: String,
    pub metric_value: f32,
    pub unit: String,
    pub tags: serde_json::Value,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeRecord {
    pub id: Uuid,
    pub from_version: String,
    pub to_version: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub metadata: serde_json::Value,
}
