//! # Nexus Server
//!
//! Main binary that orchestrates all Nexus services:
//! - REST API (HTTP)
//! - WebSocket Gateway (real-time events)
//! - Voice Server (WebRTC signaling)
//!
//! All services can run in a single process (simple deployment)
//! or be split into separate processes (horizontal scaling).

use nexus_api::{build_router, AppState};
use nexus_common::gateway_event::GatewayEvent;
use nexus_db::Database;
use nexus_gateway::GatewayState;
use std::net::SocketAddr;
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

    // === REST API Server ===
    let api_state = AppState {
        db: db.clone(),
        gateway_tx: gateway_tx.clone(),
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

    tracing::info!("📡 REST API listening on http://{api_addr}");
    tracing::info!("🔌 Gateway listening on ws://{gateway_addr}");

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
    )?;

    Ok(())
}
