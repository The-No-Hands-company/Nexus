//! Health check endpoint — for load balancers, monitoring, and Docker health checks.

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use std::sync::Arc;

use crate::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_secs: u64,
}

/// Health check router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health_check))
}

async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    // Check database connectivity
    let db_ok = nexus_db::postgres::health_check(&state.db.pg).await;

    Json(HealthResponse {
        status: if db_ok {
            "healthy".into()
        } else {
            "degraded".into()
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0, // TODO: track actual uptime
    })
}
