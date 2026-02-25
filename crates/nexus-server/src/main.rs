//! # Nexus Server
//!
//! Main binary that orchestrates all Nexus services:
//! - REST API (HTTP)
//! - WebSocket Gateway (real-time events)
//! - Voice Server (WebRTC SFU + signaling)
//!
//! All services can run in a single process (simple deployment)
//! or be split into separate processes (horizontal scaling).
//!
//! ## Lite mode
//!
//! Run `nexus serve --lite` for a zero-dependency, single-binary deployment:
//! - SQLite database (`nexus.db` in the current directory)
//! - Local filesystem uploads (`./data/uploads/`)
//! - No Docker, no MinIO, no MeiliSearch required.

use clap::{Parser, Subcommand};
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

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "nexus",
    about = "Privacy-first, community-owned chat server",
    version = env!("CARGO_PKG_VERSION"),
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the Nexus server.
    Serve {
        /// Lite mode: single binary, SQLite database, local file storage.
        /// No Docker or external services required.
        #[arg(long, env = "NEXUS_LITE", default_value_t = false)]
        lite: bool,

        /// HTTP API port (default: 8080).
        #[arg(long, env = "PORT", default_value_t = 8080)]
        port: u16,

        /// WebSocket gateway port (default: 8081).
        #[arg(long, env = "GATEWAY_PORT", default_value_t = 8081)]
        gateway_port: u16,

        /// Voice signaling port (default: 8082).
        #[arg(long, env = "VOICE_PORT", default_value_t = 8082)]
        voice_port: u16,
    },
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Serve {
            lite,
            port,
            gateway_port,
            voice_port,
        } => run_server(lite, port, gateway_port, voice_port).await,
    }
}

// ── Server startup ────────────────────────────────────────────────────────────

async fn run_server(
    lite: bool,
    port: u16,
    gateway_port: u16,
    voice_port: u16,
) -> anyhow::Result<()> {
    // ── Lite-mode environment bootstrap ──────────────────────────────────────
    // Before loading config, inject sensible defaults so the server works
    // out-of-the-box without any env vars or config files.
    if lite {
        // SQLite database in current directory
        if std::env::var("DATABASE_URL").is_err() {
            // SAFETY: single-threaded startup; no other threads are reading env yet.
            unsafe { std::env::set_var("DATABASE_URL", "sqlite://nexus.db?mode=rwc") };
        }
        // Auto-generate JWT secret on first run and store in NEXUS_JWT_SECRET
        if std::env::var("JWT_SECRET").is_err() {
            let secret = generate_or_load_lite_secret("nexus.toml")?;
            // SAFETY: single-threaded startup; no other threads are reading env yet.
            unsafe { std::env::set_var("JWT_SECRET", secret) };
        }
        // Public file URL for local uploads
        if std::env::var("NEXUS_PUBLIC_URL").is_err() {
            // SAFETY: single-threaded startup; no other threads are reading env yet.
            unsafe {
                std::env::set_var(
                    "NEXUS_PUBLIC_URL",
                    format!("http://127.0.0.1:{port}"),
                )
            };
        }
    }

    // ── Configuration ─────────────────────────────────────────────────────────
    let config = nexus_common::config::init()?;

    // ── Tracing ───────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nexus=info,tower_http=info".into()),
        )
        .with_target(!lite)          // less noisy in lite mode
        .with_thread_ids(false)
        .init();

    if lite {
        tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        tracing::info!("  Nexus v{}  —  Lite Mode", env!("CARGO_PKG_VERSION"));
        tracing::info!("  No Docker required. Data stored locally.");
        tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    } else {
        tracing::info!("🚀 Starting Nexus v{}", env!("CARGO_PKG_VERSION"));
        tracing::info!("   Privacy-first. Community-owned. No ID required.");
        tracing::info!("   ─────────────────────────────────────────────");
    }

    // ── Database ──────────────────────────────────────────────────────────────
    let db = Database::connect(config).await?;
    db.migrate().await?;
    tracing::info!("✅ Database ready");

    // ── Event bus ─────────────────────────────────────────────────────────────
    let (gateway_tx, _) = broadcast::channel::<GatewayEvent>(10_000);

    // ── Voice Server ──────────────────────────────────────────────────────────
    let local_ip: std::net::IpAddr = "127.0.0.1".parse()?;
    let voice_server = VoiceServer::new(db.clone(), gateway_tx.clone(), local_ip);
    let voice_state = voice_server.state.voice_state.clone();

    // ── Storage ───────────────────────────────────────────────────────────────
    let public_base = std::env::var("NEXUS_PUBLIC_URL")
        .unwrap_or_else(|_| format!("http://127.0.0.1:{port}"));

    let storage = if lite || config.storage.endpoint.is_empty() {
        let data_dir = &config.storage.data_dir;
        tracing::info!("📁 Local file storage at {data_dir}");
        StorageClient::new_local(data_dir, format!("{public_base}/files"))?
    } else {
        let s = StorageClient::new(&DbStorageConfig {
            endpoint: config.storage.endpoint.clone(),
            access_key: config.storage.access_key.clone(),
            secret_key: config.storage.secret_key.clone(),
            bucket: config.storage.bucket.clone(),
            region: config.storage.region.clone(),
            public_url: None,
        })?;
        s.ensure_bucket().await?;
        tracing::info!("📦 Object storage ready (bucket: {})", config.storage.bucket);
        s
    };

    // ── Search ────────────────────────────────────────────────────────────────
    let search = if !lite && !config.search.url.is_empty() {
        let s = SearchClient::new(&config.search.url, &config.search.api_key);
        s.bootstrap_indexes().await?;
        tracing::info!("🔍 MeiliSearch ready at {}", config.search.url);
        s
    } else if lite {
        let data_dir = &config.storage.data_dir;
        tracing::info!("🔍 Tantivy embedded search (lite mode)");
        SearchClient::new_tantivy(data_dir)?
    } else {
        SearchClient::disabled()
    };

    // ── Federation ────────────────────────────────────────────────────────────
    let federation_key = KeyManager::new(db.pool.clone()).load_or_generate().await?;
    tracing::info!("🔑 Federation signing key ready: {}", federation_key.key_id);
    let federation_client = Arc::new(FederationClient::new(
        &config.server.name,
        federation_key.clone(),
    ));

    // ── REST API ──────────────────────────────────────────────────────────────
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
    let host: std::net::IpAddr = "0.0.0.0".parse()?;
    let api_addr = SocketAddr::new(host, port);
    let gateway_addr = SocketAddr::new(host, gateway_port);
    let voice_addr = SocketAddr::new(host, voice_port);

    // ── WebSocket Gateway ─────────────────────────────────────────────────────
    let gateway_state = GatewayState::with_broadcast(db.clone(), gateway_tx);
    let gateway_router = nexus_gateway::build_router(gateway_state);

    // ── Voice Signaling ───────────────────────────────────────────────────────
    let voice_router = voice_server.build_router();

    if lite {
        tracing::info!("");
        tracing::info!("  ✅  Nexus is running!");
        tracing::info!("  🌐  API:     http://127.0.0.1:{port}");
        tracing::info!("  🔌  Gateway: ws://127.0.0.1:{gateway_port}");
        tracing::info!("  🎙️   Voice:   ws://127.0.0.1:{voice_port}");
        tracing::info!("");
        tracing::info!("  Open your desktop client and connect to:");
        tracing::info!("  http://127.0.0.1:{port}");
        tracing::info!("");
    } else {
        tracing::info!("📡 REST API      → http://{api_addr}");
        tracing::info!("🔌 Gateway       → ws://{gateway_addr}");
        tracing::info!("🎙️  Voice server  → ws://{voice_addr}");
    }

    tokio::try_join!(
        async {
            let listener = tokio::net::TcpListener::bind(api_addr).await?;
            axum::serve(listener, api_router).await?;
            Ok::<_, anyhow::Error>(())
        },
        async {
            let listener = tokio::net::TcpListener::bind(gateway_addr).await?;
            axum::serve(listener, gateway_router).await?;
            Ok::<_, anyhow::Error>(())
        },
        async {
            let listener = tokio::net::TcpListener::bind(voice_addr).await?;
            axum::serve(listener, voice_router).await?;
            Ok::<_, anyhow::Error>(())
        },
    )?;

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Load the JWT secret from `nexus.toml`, or generate and persist a new one.
/// The file is a minimal TOML with a single `jwt_secret` key so it survives
/// across restarts without any additional config.
fn generate_or_load_lite_secret(path: &str) -> anyhow::Result<String> {
    use std::io::{Read, Write};

    let key = "jwt_secret";

    // Try to read existing secret
    if let Ok(mut f) = std::fs::File::open(path) {
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;
        for line in contents.lines() {
            if let Some(rest) = line.strip_prefix("jwt_secret = \"") {
                if let Some(secret) = rest.strip_suffix('"') {
                    if !secret.is_empty() {
                        return Ok(secret.to_string());
                    }
                }
            }
        }
    }

    // Generate a new 64-byte hex secret
    use rand::RngCore;
    let mut bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut bytes);
    let secret = hex::encode(bytes);

    // Persist — append/create
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "\n{key} = \"{secret}\"")?;

    tracing::info!("🔐 Generated JWT secret → {path}  (keep this file safe)");
    Ok(secret)
}
