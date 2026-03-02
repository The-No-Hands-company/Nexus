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
    http::{header, HeaderMap, StatusCode},
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
///
/// # Access control
///
/// If the environment variable `NEXUS_METRICS_TOKEN` is set, the endpoint
/// requires `Authorization: Bearer <token>`.  If it is **not** set, the
/// endpoint is restricted to loopback addresses only (127.0.0.1 / ::1),
/// checked via `X-Forwarded-For` / `X-Real-IP` (same logic as the auth
/// rate limiter).
///
/// Set `NEXUS_METRICS_TOKEN=your-secret` for external Prometheus scrapers.
async fn metrics_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    // ── Access check ──────────────────────────────────────────────────────
    if let Ok(expected_token) = std::env::var("NEXUS_METRICS_TOKEN") {
        // Token-gated: any IP is allowed if the bearer token matches.
        let provided = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        if provided != expected_token {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Provide Authorization: Bearer <NEXUS_METRICS_TOKEN>",
            ));
        }
    } else {
        // No token configured: restrict to loopback only.
        let ip = crate::middleware::extract_client_ip(&headers);
        let is_loopback = ip == "unknown"
            || ip == "127.0.0.1"
            || ip == "::1"
            || ip.starts_with("::ffff:127.");
        if !is_loopback {
            return Err((
                StatusCode::FORBIDDEN,
                "Set NEXUS_METRICS_TOKEN to enable remote scraping",
            ));
        }
    }

    let body = state.prometheus.render();
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        body,
    ))
}
