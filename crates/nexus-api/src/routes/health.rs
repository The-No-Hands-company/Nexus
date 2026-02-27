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
//!     "search":   { "status": "ok", "backend": "tantivy", "latency_ms": 0 }
//!   }
//! }
//! ```
//!
//! The top-level `status` is `"degraded"` if any individual check reports `"error"`.

use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;
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
}

#[derive(Serialize)]
struct Checks {
    database: CheckResult,
    redis: CheckResult,
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
        match tokio::time::timeout(timeout, nexus_db::postgres::health_check(&state.db.pool)).await {
            Ok(true)  => CheckResult { status: "ok", latency_ms: Some(t.elapsed().as_millis() as u64), backend: None, error: None },
            Ok(false) => CheckResult { status: "error", latency_ms: None, backend: None, error: Some("ping query failed".into()) },
            Err(_)    => CheckResult { status: "error", latency_ms: None, backend: None, error: Some("timeout".into()) },
        }
    };

    // ── Redis ─────────────────────────────────────────────────────────────────
    let redis_check = {
        let t = Instant::now();
        match tokio::time::timeout(timeout, state.db.health_redis()).await {
            Ok(Some(true))  => CheckResult { status: "ok", latency_ms: Some(t.elapsed().as_millis() as u64), backend: None, error: None },
            Ok(Some(false)) => CheckResult { status: "error", latency_ms: None, backend: None, error: Some("PING failed".into()) },
            Ok(None)        => CheckResult { status: "not_configured", latency_ms: None, backend: None, error: None },
            Err(_)          => CheckResult { status: "error", latency_ms: None, backend: None, error: Some("timeout".into()) },
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
            (true, _)          => "ok",
            (false, _)         => "error",
        };
        CheckResult {
            status,
            latency_ms: if ok { Some(t.elapsed().as_millis() as u64) } else { None },
            backend: Some(backend_name),
            error: err,
        }
    };

    // ── Aggregate ─────────────────────────────────────────────────────────────
    let degraded = db_check.status == "error"
        || redis_check.status == "error"
        || search_check.status == "error";

    let http_status = if degraded { StatusCode::SERVICE_UNAVAILABLE } else { StatusCode::OK };

    (
        http_status,
        Json(HealthResponse {
            status: if degraded { "degraded" } else { "healthy" },
            version: env!("CARGO_PKG_VERSION"),
            uptime_secs: state.started_at.elapsed().as_secs(),
            checks: Checks { database: db_check, redis: redis_check, search: search_check },
        }),
    )
}
