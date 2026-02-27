//! Prometheus metrics endpoint — exposes `GET /metrics` for scraping.
//!
//! Metrics exposed:
//!   - `nexus_http_requests_total{method, path, status}` — request counter
//!   - `nexus_http_request_duration_seconds{method, path}` — latency histogram
//!   - `nexus_voice_users_active` — gauge: users currently in a voice channel
//!
//! The HTTP request metrics are updated by the `record_request_metrics` middleware
//! in `middleware.rs`.  Voice gauges are updated directly in `nexus-voice/src/state.rs`
//! on join / leave.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::Arc;

use crate::AppState;

/// Mount the `/metrics` route.
///
/// The endpoint is intentionally outside `/api/v1` so standard Prometheus
/// `scrape_configs` can use `metrics_path: /metrics` without a prefix.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/metrics", get(metrics_handler))
}

/// Render current Prometheus metrics in the text exposition format 0.0.4.
async fn metrics_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let body = state.prometheus.render();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}
