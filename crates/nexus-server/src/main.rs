//! # Nexus Server
//!
//! Main binary that orchestrates all Nexus services:
//! - REST API (HTTP)
//! - WebSocket Gateway (real-time events)
//! - Voice Server (WebRTC SFU + signaling)
//!
//! All services can run in a single process (simple deployment)
//! or be split into separate processes (horizontal scaling).

use nexus_api::{build_router, AppState};
use nexus_common::gateway_event::GatewayEvent;
use nexus_db::{
    search::SearchClient,
    storage::{StorageClient, StorageConfig as DbStorageConfig},
    Database,
};
use nexus_federation::{FederationClient, KeyManager};
use nexus_gateway::GatewayState;
use nexus_voice::VoiceServer;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = nexus_common::config::init()?;

    // Initialize tracing (structured logging)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nexus=debug,tower_http=debug".into()),
        )
        .with_target(true)
        .with_thread_ids(true)
        .init();

    tracing::info!("🚀 Starting Nexus v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("   Privacy-first. Community-owned. No ID required.");
    tracing::info!("   ─────────────────────────────────────────────");

    // Connect to databases
    let db = Database::connect(config).await?;

    // Run migrations
    db.migrate().await?;

    // === Shared event broadcast channel ===
    // This is the bridge between REST API mutations and WebSocket gateway.
    // When the API creates/updates/deletes a message, it sends a GatewayEvent
    // through this channel, which the gateway then forwards to connected clients.
    let (gateway_tx, _) = broadcast::channel::<GatewayEvent>(10_000);

    // === Voice Server ===
    // WebRTC SFU for voice/video, with its own WebSocket for signaling.
    let local_ip: std::net::IpAddr = config.server.host.parse()?;
    let voice_server = VoiceServer::new(db.clone(), gateway_tx.clone(), local_ip);
    let voice_state = voice_server.state.voice_state.clone();

    // === Object Storage (MinIO / S3) ===
    let storage = StorageClient::new(&DbStorageConfig {
        endpoint: config.storage.endpoint.clone(),
        access_key: config.storage.access_key.clone(),
        secret_key: config.storage.secret_key.clone(),
        bucket: config.storage.bucket.clone(),
        region: config.storage.region.clone(),
        public_url: None,
    })?;
    storage.ensure_bucket().await?;
    tracing::info!("📦 Object storage ready (bucket: {})", config.storage.bucket);

    // === MeiliSearch ===
    let search = SearchClient::new(&config.search.url, &config.search.api_key);
    search.bootstrap_indexes().await?;
    tracing::info!("🔍 MeiliSearch ready at {}", config.search.url);

    // === Federation signing key ===
    // Load the active Ed25519 key from DB, or generate + persist a new one on first run.
    let federation_key = KeyManager::new(db.pg.clone()).load_or_generate().await?;
    tracing::info!("🔑 Federation signing key ready: {}", federation_key.key_id);
    let federation_client = Arc::new(FederationClient::new(
        &config.server.name,
        federation_key.clone(),
    ));

    // === REST API Server ===
    let api_state = AppState {
        db: db.clone(),
        gateway_tx: gateway_tx.clone(),
        voice_state: voice_state.clone(),
        storage,
        search,
        server_name: config.server.name.clone(),
        federation_key,
        federation_client,
    };
    let api_router = build_router(api_state);
    let api_addr = SocketAddr::new(
        config.server.host.parse()?,
        config.server.port,
    );

    // === WebSocket Gateway ===
    let gateway_state = GatewayState::with_broadcast(db.clone(), gateway_tx);
    let gateway_router = nexus_gateway::build_router(gateway_state);
    let gateway_addr = SocketAddr::new(
        config.server.host.parse()?,
        config.server.gateway_port,
    );

    // === Voice Signaling WebSocket ===
    let voice_router = voice_server.build_router();
    let voice_addr = SocketAddr::new(
        config.server.host.parse()?,
        config.server.voice_port,
    );

    tracing::info!("📡 REST API listening on http://{api_addr}");
    tracing::info!("🔌 Gateway listening on ws://{gateway_addr}");
    tracing::info!("🎙️  Voice server listening on ws://{voice_addr}");

    // Run all servers concurrently
    tokio::try_join!(
        // REST API
        async {
            let listener = tokio::net::TcpListener::bind(api_addr).await?;
            axum::serve(listener, api_router).await?;
            Ok::<_, anyhow::Error>(())
        },
        // WebSocket Gateway
        async {
            let listener = tokio::net::TcpListener::bind(gateway_addr).await?;
            axum::serve(listener, gateway_router).await?;
            Ok::<_, anyhow::Error>(())
        },
        // Voice Signaling
        async {
            let listener = tokio::net::TcpListener::bind(voice_addr).await?;
            axum::serve(listener, voice_router).await?;
            Ok::<_, anyhow::Error>(())
        },
    )?;

    Ok(())
}
