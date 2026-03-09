//! Repository: scaling_metrics + upgrade_records — Phase 20-05.

use nexus_common::models::scalability::{ScalingMetric, UpgradeRecord};
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::{SCALING_METRIC_COLS, UPGRADE_RECORD_COLS};

// ── Scaling Metrics ───────────────────────────────────────────────────────

pub async fn record_metric(
    pool: &AnyPool,
    id: Uuid,
    instance_id: &str,
    metric_name: &str,
    metric_value: f32,
    unit: &str,
    tags: &serde_json::Value,
) -> Result<ScalingMetric, sqlx::Error> {
    let q = format!(
        "INSERT INTO scaling_metrics (id, instance_id, metric_name, metric_value, unit, tags) \
         VALUES ($1, $2, $3, $4, $5, $6::jsonb) \
         RETURNING {SCALING_METRIC_COLS}"
    );
    sqlx::query_as::<_, ScalingMetric>(&q)
        .bind(id.to_string())
        .bind(instance_id)
        .bind(metric_name)
        .bind(metric_value as f64)
        .bind(unit)
        .bind(tags.to_string())
        .fetch_one(pool)
        .await
}

pub async fn list_metrics(
    pool: &AnyPool,
    instance_id: &str,
    limit: i64,
) -> Result<Vec<ScalingMetric>, sqlx::Error> {
    let q = format!(
        "SELECT {SCALING_METRIC_COLS} FROM scaling_metrics \
         WHERE instance_id = $1 ORDER BY recorded_at DESC LIMIT $2"
    );
    sqlx::query_as::<_, ScalingMetric>(&q)
        .bind(instance_id)
        .bind(limit)
        .fetch_all(pool)
        .await
}

// ── Upgrade Records ───────────────────────────────────────────────────────

pub async fn create_upgrade_record(
    pool: &AnyPool,
    id: Uuid,
    from_version: &str,
    to_version: &str,
    notes: Option<&str>,
    metadata: &serde_json::Value,
) -> Result<UpgradeRecord, sqlx::Error> {
    let q = format!(
        "INSERT INTO upgrade_records (id, from_version, to_version, notes, metadata) \
         VALUES ($1, $2, $3, $4, $5::jsonb) \
         RETURNING {UPGRADE_RECORD_COLS}"
    );
    sqlx::query_as::<_, UpgradeRecord>(&q)
        .bind(id.to_string())
        .bind(from_version)
        .bind(to_version)
        .bind(notes)
        .bind(metadata.to_string())
        .fetch_one(pool)
        .await
}

pub async fn list_upgrade_records(
    pool: &AnyPool,
) -> Result<Vec<UpgradeRecord>, sqlx::Error> {
    let q = format!(
        "SELECT {UPGRADE_RECORD_COLS} FROM upgrade_records ORDER BY started_at DESC"
    );
    sqlx::query_as::<_, UpgradeRecord>(&q)
        .fetch_all(pool)
        .await
}

pub async fn complete_upgrade(
    pool: &AnyPool,
    id: Uuid,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE upgrade_records SET status = $2, completed_at = now() WHERE id = $1")
        .bind(id.to_string())
        .bind(status)
        .execute(pool)
        .await?;
    Ok(())
}
