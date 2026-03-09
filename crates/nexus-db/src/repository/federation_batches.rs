//! Repository: federation_batches — Phase 20-02.

use nexus_common::models::scalability::{FederationEventBatch, FederationRoute, FederationDedupEntry};
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::{FEDERATION_EVENT_BATCH_COLS, FEDERATION_ROUTE_COLS, FEDERATION_DEDUP_COLS};

// ── Event Batches ─────────────────────────────────────────────────────────

pub async fn create_event_batch(
    pool: &AnyPool,
    id: Uuid,
    target_instance: &str,
    events: &serde_json::Value,
    event_count: i32,
) -> Result<FederationEventBatch, sqlx::Error> {
    let q = format!(
        "INSERT INTO federation_event_batches (id, target_instance, events, event_count) \
         VALUES ($1, $2, $3::jsonb, $4) \
         RETURNING {FEDERATION_EVENT_BATCH_COLS}"
    );
    sqlx::query_as::<_, FederationEventBatch>(&q)
        .bind(id.to_string())
        .bind(target_instance)
        .bind(events.to_string())
        .bind(event_count)
        .fetch_one(pool)
        .await
}

pub async fn list_pending_batches(
    pool: &AnyPool,
    limit: i64,
) -> Result<Vec<FederationEventBatch>, sqlx::Error> {
    let q = format!(
        "SELECT {FEDERATION_EVENT_BATCH_COLS} FROM federation_event_batches \
         WHERE status = 'pending' ORDER BY created_at ASC LIMIT $1"
    );
    sqlx::query_as::<_, FederationEventBatch>(&q)
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn mark_batch_sent(
    pool: &AnyPool,
    id: Uuid,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE federation_event_batches SET status = $2, sent_at = now() WHERE id = $1"
    )
        .bind(id.to_string())
        .bind(status)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Federation Routes ─────────────────────────────────────────────────────

pub async fn upsert_federation_route(
    pool: &AnyPool,
    id: Uuid,
    source_instance: &str,
    target_instance: &str,
    latency_ms: i32,
    is_websocket: bool,
    priority: i32,
    status: &str,
) -> Result<FederationRoute, sqlx::Error> {
    let q = format!(
        "INSERT INTO federation_routes \
         (id, source_instance, target_instance, latency_ms, is_websocket, priority, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (source_instance, target_instance) DO UPDATE SET \
           latency_ms = $4, is_websocket = $5, priority = $6, status = $7, last_probed = now() \
         RETURNING {FEDERATION_ROUTE_COLS}"
    );
    sqlx::query_as::<_, FederationRoute>(&q)
        .bind(id.to_string())
        .bind(source_instance)
        .bind(target_instance)
        .bind(latency_ms)
        .bind(is_websocket)
        .bind(priority)
        .bind(status)
        .fetch_one(pool)
        .await
}

pub async fn list_federation_routes(
    pool: &AnyPool,
) -> Result<Vec<FederationRoute>, sqlx::Error> {
    let q = format!(
        "SELECT {FEDERATION_ROUTE_COLS} FROM federation_routes ORDER BY priority DESC, latency_ms ASC"
    );
    sqlx::query_as::<_, FederationRoute>(&q)
        .fetch_all(pool)
        .await
}

// ── Dedup Log ─────────────────────────────────────────────────────────────

pub async fn check_dedup(
    pool: &AnyPool,
    event_id: &str,
    source_instance: &str,
) -> Result<bool, sqlx::Error> {
    let row: Option<FederationDedupEntry> = sqlx::query_as::<_, FederationDedupEntry>(
        &format!("SELECT {FEDERATION_DEDUP_COLS} FROM federation_dedup_log WHERE event_id = $1 AND source_instance = $2")
    )
        .bind(event_id)
        .bind(source_instance)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

pub async fn insert_dedup(
    pool: &AnyPool,
    event_id: &str,
    source_instance: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO federation_dedup_log (event_id, source_instance) VALUES ($1, $2) ON CONFLICT DO NOTHING"
    )
        .bind(event_id)
        .bind(source_instance)
        .execute(pool)
        .await?;
    Ok(())
}
