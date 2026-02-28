//! # nexus-api
//!
//! REST API layer for Nexus. Provides HTTP endpoints for all CRUD operations,
//! authentication, and client-facing functionality.

pub mod auth;
pub mod middleware;
pub mod routes;

use axum::Router;
use metrics_exporter_prometheus::PrometheusHandle;
use nexus_common::gateway_event::GatewayEvent;
use nexus_db::{search::SearchClient, storage::StorageClient, Database};
use nexus_federation::{client::FederationClient, ServerKeyPair};
use nexus_voice::state::VoiceStateManager;
use std::{sync::Arc, time::Instant};
use tokio::sync::broadcast;

/// Shared application state available to all route handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    /// Broadcast sender to push events to the WebSocket gateway.
    /// API mutations (message create, channel update, etc.) use this
    /// to notify all connected clients in real-time.
    pub gateway_tx: broadcast::Sender<GatewayEvent>,
    /// Voice state manager — shared with the voice server for REST-based
    /// voice operations (state queries, moderation actions).
    pub voice_state: VoiceStateManager,
    /// MinIO / S3-compatible object storage client for file uploads.
    pub storage: StorageClient,
    /// MeiliSearch client for full-text message search.
    pub search: SearchClient,
    // ── v0.8 Federation ──────────────────────────────────────────────────────
    /// Public server name used in federation (e.g. "nexus.example.com").
    pub server_name: String,
    /// Active Ed25519 signing key for all outbound federation requests.
    pub federation_key: Arc<ServerKeyPair>,
    /// Signed HTTP client for outbound server-to-server federation requests.
    pub federation_client: Arc<FederationClient>,
    /// Monotonic timestamp captured at process start — used to compute uptime.
    pub started_at: Instant,
    /// Prometheus metrics handle — render current metrics text with `prometheus.render()`.
    pub prometheus: PrometheusHandle,
}

/// Build the complete API router with all routes and middleware.
pub fn build_router(state: AppState) -> Router {
    // Create the Arc up-front so we can share it as an Axum Extension
    // (`combined_auth_middleware` reads it to look up bot tokens).
    let arc_state = Arc::new(state);

    let api_routes = Router::new()
        .merge(routes::auth::router())
        .merge(routes::users::router())
        .merge(routes::servers::router())
        .merge(routes::channels::router())
        .merge(routes::messages::router())
        .merge(routes::dms::router())
        .merge(routes::voice::router())
        .merge(routes::health::router())
        // v0.4 Rich Features
        .merge(routes::uploads::router())
        .merge(routes::threads::router())
        .merge(routes::emoji::router())
        .merge(routes::search::router())
        .merge(routes::presence::router())
        .merge(routes::relationships::router())
        // v0.5 Encryption
        .merge(routes::keys::router())
        .merge(routes::e2ee::router())
        .merge(routes::verification::router())
        // v0.7 Extensibility
        .merge(routes::bots::router())
        .merge(routes::webhooks::router())
        .merge(routes::slash_commands::router())
        .merge(routes::extensibility::router())
        // v0.8 Federation — client-facing directory endpoints
        .merge(routes::directory::router())
        // v0.9.7 Account Security
        .merge(routes::two_fa::router())           // authenticated 2FA management
        .merge(routes::two_fa::public_router())    // unauthenticated MFA verify endpoint
        .merge(routes::sessions::router())         // session listing + revocation
        .merge(routes::email_verification::router()) // email verify + resend
        // v0.9.8 Moderation & Safety
        .merge(routes::moderation::router())       // audit log, kick, ban, timeout, reports, word filters
        // v0.12 Channel Type Completion
        .merge(routes::forum::router())            // forum posts + tags
        .merge(routes::stages::router())           // stage instances + speaker management
        // v0.13 Engagement Features
        .merge(routes::polls::router())                       // polls — create, vote, results, end
        .merge(routes::scheduled_messages::router())          // scheduled messages CRUD
        .merge(routes::bookmarks::router())                   // message bookmarks
        .merge(routes::drafts::router())                      // per-channel drafts
        // v0.14 Platform Differentiation
        .merge(routes::forward::router())                         // message forwarding
        .merge(routes::events::router())                          // server events + RSVP
        .merge(routes::stickers::router())                        // sticker packs
        .merge(routes::inline_query::router())                    // inline bot suggestions
        // v0.15 Community Ecosystem
        .merge(routes::badges::router())                          // user badges
        .merge(routes::boosters::router())                        // server supporter tiers
        .merge(routes::canvas::router())                          // canvas document channels
        // v0.8.5 Federation UX — admin management + cross-instance search
        .merge(routes::federation_admin::router())                // federation admin panel API
        // Make Arc<AppState> available as an Axum Extension so that
        // `combined_auth_middleware` can perform DB lookups for bot tokens
        // without requiring `from_fn_with_state` on every sub-router.
        .layer(axum::Extension(arc_state.clone()));

    Router::new()
        .nest("/api/v1", api_routes)
        // v0.8 Federation — server-to-server endpoints (live outside /api/v1)
        .merge(routes::federation::federation_router())
        // Prometheus metrics — at /metrics (outside /api/v1 for standard scrape_configs)
        .merge(routes::metrics::router())
        // Local file serving (lite mode — no-op in full mode)
        .merge(routes::files::router())
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                // Emit a span per request with method + URI path as structured fields
                .make_span_with(
                    tower_http::trace::DefaultMakeSpan::new()
                        .level(tracing::Level::INFO)
                        .include_headers(false),
                )
                // Emit a record per response with status + latency_ms as structured fields
                .on_response(
                    tower_http::trace::DefaultOnResponse::new()
                        .level(tracing::Level::INFO)
                        .latency_unit(tower_http::LatencyUnit::Millis),
                )
                // Emit a record on request body failures
                .on_failure(
                    tower_http::trace::DefaultOnFailure::new()
                        .level(tracing::Level::ERROR),
                ),
        )
        .layer(tower_http::compression::CompressionLayer::new())
        .layer(axum::middleware::from_fn(middleware::security_headers))
        .layer(axum::middleware::from_fn(middleware::record_request_metrics))
        .with_state(arc_state)
}

