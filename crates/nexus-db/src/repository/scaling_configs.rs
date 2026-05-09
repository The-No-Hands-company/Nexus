//! Repository: scaling_configs — Phase 20-01.

use nexus_common::models::scalability::ScalingConfig;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::SCALING_CONFIG_COLS;

pub async fn upsert_scaling_config(
    pool: &AnyPool,
    id: Uuid,
    instance_id: &str,
    region: &str,
    shard_strategy: &str,
    redis_mode: &str,
    gateway_weight: i32,
    max_connections: i32,
    metadata: &serde_json::Value,
    is_active: bool,
) -> Result<ScalingConfig, sqlx::Error> {
    let q = format!(
        "INSERT INTO scaling_configs \
         (id, instance_id, region, shard_strategy, redis_mode, gateway_weight, \
          max_connections, metadata, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9) \
         ON CONFLICT (instance_id) DO UPDATE SET \
           region = $3, shard_strategy = $4, redis_mode = $5, \
           gateway_weight = $6, max_connections = $7, metadata = $8::jsonb, \
           is_active = $9, updated_at = now() \
         RETURNING {SCALING_CONFIG_COLS}"
    );
    sqlx::query_as::<_, ScalingConfig>(&q)
        .bind(id.to_string())
        .bind(instance_id)
        .bind(region)
        .bind(shard_strategy)
        .bind(redis_mode)
        .bind(gateway_weight)
        .bind(max_connections)
        .bind(metadata.to_string())
        .bind(is_active)
        .fetch_one(pool)
        .await
}

pub async fn list_scaling_configs(pool: &AnyPool) -> Result<Vec<ScalingConfig>, sqlx::Error> {
    let q =
        format!("SELECT {SCALING_CONFIG_COLS} FROM scaling_configs ORDER BY region, instance_id");
    sqlx::query_as::<_, ScalingConfig>(&q).fetch_all(pool).await
}

pub async fn get_scaling_config(
    pool: &AnyPool,
    id: Uuid,
) -> Result<Option<ScalingConfig>, sqlx::Error> {
    let q = format!("SELECT {SCALING_CONFIG_COLS} FROM scaling_configs WHERE id = $1");
    sqlx::query_as::<_, ScalingConfig>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn delete_scaling_config(pool: &AnyPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM scaling_configs WHERE id = $1")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}
