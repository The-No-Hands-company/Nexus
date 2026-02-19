//! # nexus-api
//!
//! REST API layer for Nexus. Provides HTTP endpoints for all CRUD operations,
//! authentication, and client-facing functionality.

pub mod auth;
pub mod middleware;
pub mod routes;

use axum::Router;
use nexus_common::gateway_event::GatewayEvent;
use nexus_db::{search::SearchClient, storage::StorageClient, Database};
use nexus_federation::{client::FederationClient, ServerKeyPair};
use nexus_voice::state::VoiceStateManager;
use std::sync::Arc;
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
}

/// Build the complete API router with all routes and middleware.
pub fn build_router(state: AppState) -> Router {
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
        .merge(routes::directory::router());

    Router::new()
        .nest("/api/v1", api_routes)
        // v0.8 Federation — server-to-server endpoints (live outside /api/v1)
        .merge(routes::federation::federation_router())
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::compression::CompressionLayer::new())
        .with_state(Arc::new(state))
}

