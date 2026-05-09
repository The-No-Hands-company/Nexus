//! Repository: sfu_nodes — Phase 20-01/20-03.

use nexus_common::models::scalability::SfuNode;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::SFU_NODE_COLS;

pub async fn upsert_sfu_node(
    pool: &AnyPool,
    id: Uuid,
    instance_id: &str,
    region: &str,
    hostname: &str,
    port: i32,
    capacity: i32,
    status: &str,
    metadata: &serde_json::Value,
) -> Result<SfuNode, sqlx::Error> {
    let q = format!(
        "INSERT INTO sfu_nodes \
         (id, instance_id, region, hostname, port, capacity, status, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb) \
         ON CONFLICT (instance_id) DO UPDATE SET \
           region = $3, hostname = $4, port = $5, capacity = $6, \
           status = $7, metadata = $8::jsonb, last_heartbeat = now() \
         RETURNING {SFU_NODE_COLS}"
    );
    sqlx::query_as::<_, SfuNode>(&q)
        .bind(id.to_string())
        .bind(instance_id)
        .bind(region)
        .bind(hostname)
        .bind(port)
        .bind(capacity)
        .bind(status)
        .bind(metadata.to_string())
        .fetch_one(pool)
        .await
}

pub async fn list_sfu_nodes(
    pool: &AnyPool,
    region: Option<&str>,
) -> Result<Vec<SfuNode>, sqlx::Error> {
    if let Some(r) = region {
        let q = format!(
            "SELECT {SFU_NODE_COLS} FROM sfu_nodes WHERE region = $1 ORDER BY current_load ASC"
        );
        sqlx::query_as::<_, SfuNode>(&q)
            .bind(r)
            .fetch_all(pool)
            .await
    } else {
        let q = format!("SELECT {SFU_NODE_COLS} FROM sfu_nodes ORDER BY region, current_load ASC");
        sqlx::query_as::<_, SfuNode>(&q).fetch_all(pool).await
    }
}

pub async fn update_sfu_load(
    pool: &AnyPool,
    id: Uuid,
    current_load: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sfu_nodes SET current_load = $2, last_heartbeat = now() WHERE id = $1")
        .bind(id.to_string())
        .bind(current_load)
        .execute(pool)
        .await?;
    Ok(())
}
