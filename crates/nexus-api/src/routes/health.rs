//! Health check endpoint — for load balancers, monitoring, and Docker health checks.
//!
//! `GET /api/v1/health` returns a structured JSON payload that reflects the live
//! state of every critical dependency:
//!
//! ```json
//! {
//!   "status": "healthy",
//!   "version": "0.1.0",
//!   "uptime_secs": 42,
//!   "checks": {
//!     "database": { "status": "ok", "latency_ms": 2 },
//!     "redis":    { "status": "not_configured" },
//!     "scylla":   { "status": "ok", "backend": "scylla+outbox", "pending": 0 },
//!     "search":   { "status": "ok", "backend": "tantivy", "latency_ms": 0 }
//!   }
//! }
//! ```
//!
//! The top-level `status` is `"degraded"` if any individual check reports `"error"`.

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use scylla::SessionBuilder;
use serde::Serialize;
use sqlx::AnyPool;
use std::sync::Arc;
use std::time::Instant;

use crate::AppState;

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct CheckResult {
    /// `"ok"` | `"error"` | `"not_configured"` | `"disabled"`
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
    /// Which search backend is active (meilisearch | tantivy | disabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    backend: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pending: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dispatched: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failed: Option<i64>,
}

#[derive(Default)]
struct ScyllaOutboxStats {
    pending: i64,
    dispatched: i64,
    failed: i64,
}

#[derive(Serialize)]
struct Checks {
    database: CheckResult,
    redis: CheckResult,
    scylla: CheckResult,
    search: CheckResult,
}

#[derive(Serialize)]
struct HealthResponse {
    /// `"healthy"` when all checks pass; `"degraded"` when one or more fail.
    status: &'static str,
    version: &'static str,
    uptime_secs: u64,
    checks: Checks,
}

// ── Router ────────────────────────────────────────────────────────────────────

/// Health check router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health_check))
}

// ── Handler ───────────────────────────────────────────────────────────────────

async fn health_check(State(state): State<Arc<AppState>>) -> (StatusCode, Json<HealthResponse>) {
    let timeout = std::time::Duration::from_secs(3);

    // ── Database ──────────────────────────────────────────────────────────────
    let db_check = {
        let t = Instant::now();
        match tokio::time::timeout(timeout, nexus_db::postgres::health_check(&state.db.pool)).await
        {
            Ok(true) => CheckResult {
                status: "ok",
                latency_ms: Some(t.elapsed().as_millis() as u64),
                backend: None,
                error: None,
                pending: None,
                dispatched: None,
                failed: None,
            },
            Ok(false) => CheckResult {
                status: "error",
                latency_ms: None,
                backend: None,
                error: Some("ping query failed".into()),
                pending: None,
                dispatched: None,
                failed: None,
            },
            Err(_) => CheckResult {
                status: "error",
                latency_ms: None,
                backend: None,
                error: Some("timeout".into()),
                pending: None,
                dispatched: None,
                failed: None,
            },
        }
    };

    // ── Redis ─────────────────────────────────────────────────────────────────
    let redis_check = {
        let t = Instant::now();
        match tokio::time::timeout(timeout, state.db.health_redis()).await {
            Ok(Some(true)) => CheckResult {
                status: "ok",
                latency_ms: Some(t.elapsed().as_millis() as u64),
                backend: None,
                error: None,
                pending: None,
                dispatched: None,
                failed: None,
            },
            Ok(Some(false)) => CheckResult {
                status: "error",
                latency_ms: None,
                backend: None,
                error: Some("PING failed".into()),
                pending: None,
                dispatched: None,
                failed: None,
            },
            Ok(None) => CheckResult {
                status: "not_configured",
                latency_ms: None,
                backend: None,
                error: None,
                pending: None,
                dispatched: None,
                failed: None,
            },
            Err(_) => CheckResult {
                status: "error",
                latency_ms: None,
                backend: None,
                error: Some("timeout".into()),
                pending: None,
                dispatched: None,
                failed: None,
            },
        }
    };

    // ── Scylla ────────────────────────────────────────────────────────────────
    let scylla_check = if state.db.scylla_enabled {
        let t = Instant::now();
        match tokio::time::timeout(timeout, scylla_health(&state)).await {
            Ok(Ok(stats)) => CheckResult {
                status: "ok",
                latency_ms: Some(t.elapsed().as_millis() as u64),
                backend: Some("scylla+outbox"),
                error: None,
                pending: Some(stats.pending),
                dispatched: Some(stats.dispatched),
                failed: Some(stats.failed),
            },
            Ok(Err(err)) => CheckResult {
                status: "error",
                latency_ms: None,
                backend: Some("scylla+outbox"),
                error: Some(err),
                pending: None,
                dispatched: None,
                failed: None,
            },
            Err(_) => CheckResult {
                status: "error",
                latency_ms: None,
                backend: Some("scylla+outbox"),
                error: Some("timeout".into()),
                pending: None,
                dispatched: None,
                failed: None,
            },
        }
    } else {
        CheckResult {
            status: "disabled",
            latency_ms: None,
            backend: Some("scylla"),
            error: None,
            pending: None,
            dispatched: None,
            failed: None,
        }
    };

    // ── Search ────────────────────────────────────────────────────────────────
    let search_check = {
        let t = Instant::now();
        let (ok, backend_name, err) = tokio::time::timeout(timeout, state.search.health_check())
            .await
            .unwrap_or((false, "unknown", Some("timeout".to_string())));
        let status = match (ok, backend_name) {
            (true, "disabled") => "disabled",
            (true, _) => "ok",
            (false, _) => "error",
        };
        CheckResult {
            status,
            latency_ms: if ok {
                Some(t.elapsed().as_millis() as u64)
            } else {
                None
            },
            backend: Some(backend_name),
            error: err,
            pending: None,
            dispatched: None,
            failed: None,
        }
    };

    // ── Aggregate ─────────────────────────────────────────────────────────────
    let degraded = db_check.status == "error"
        || redis_check.status == "error"
        || scylla_check.status == "error"
        || search_check.status == "error";

    let http_status = if degraded {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (
        http_status,
        Json(HealthResponse {
            status: if degraded { "degraded" } else { "healthy" },
            version: env!("CARGO_PKG_VERSION"),
            uptime_secs: state.started_at.elapsed().as_secs(),
            checks: Checks {
                database: db_check,
                redis: redis_check,
                scylla: scylla_check,
                search: search_check,
            },
        }),
    )
}

async fn scylla_outbox_stats(pool: &AnyPool) -> Result<ScyllaOutboxStats, String> {
    let pending = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM scylla_message_outbox WHERE status = 'pending'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("pending_count_failed: {e}"))?;

    let dispatched = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM scylla_message_outbox WHERE status = 'dispatched'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("dispatched_count_failed: {e}"))?;

    let failed = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM scylla_message_outbox WHERE status = 'failed'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("failed_count_failed: {e}"))?;

    Ok(ScyllaOutboxStats {
        pending,
        dispatched,
        failed,
    })
}

async fn scylla_health(state: &AppState) -> Result<ScyllaOutboxStats, String> {
    let nodes: Vec<String> = state
        .db
        .scylla_nodes
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect();

    if nodes.is_empty() {
        return Err("no_scylla_nodes_configured".into());
    }

    let mut builder = SessionBuilder::new();
    for node in &nodes {
        builder = builder.known_node(node);
    }

    let session = builder
        .build()
        .await
        .map_err(|e| format!("connect_failed: {e}"))?;

    session
        .query_unpaged("SELECT release_version FROM system.local", ())
        .await
        .map_err(|e| format!("probe_failed: {e}"))?;

    scylla_outbox_stats(&state.db.pool).await
}
