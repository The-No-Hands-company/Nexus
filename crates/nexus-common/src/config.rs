//! Application configuration loaded from environment variables and config files.
//!
//! Supports `.env` files for development and environment variables for production.
//! Config precedence: env vars > .env file > config.toml > defaults

use serde::Deserialize;
use std::sync::OnceLock;

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

/// Get the global application configuration.
///
/// # Panics
/// Panics if config has not been initialized via [`init`].
pub fn get() -> &'static AppConfig {
    CONFIG.get().expect("Config not initialized. Call nexus_common::config::init() first.")
}

/// Initialize the global configuration from environment.
///
/// Should be called once at application startup, before any other code accesses config.
pub fn init() -> Result<&'static AppConfig, config::ConfigError> {
    // Load .env file if present (development)
    let _ = dotenvy::dotenv();

    let cfg = config::Config::builder()
        // Defaults
        .set_default("server.host", "0.0.0.0")?
        .set_default("server.port", 8080)?
        .set_default("server.gateway_port", 8081)?
        .set_default("server.voice_port", 8082)?
        .set_default("server.federation_port", 8448)?
        .set_default("server.name", "localhost")?
        .set_default("database.max_connections", 20)?
        .set_default("database.min_connections", 5)?
        .set_default("auth.access_token_ttl_secs", 900)? // 15 min
        .set_default("auth.refresh_token_ttl_secs", 2_592_000)? // 30 days
        .set_default("storage.endpoint", "")?
        .set_default("storage.bucket", "nexus")?
        .set_default("storage.access_key", "")?
        .set_default("storage.secret_key", "")?
        .set_default("storage.region", "us-east-1")?
        .set_default("storage.data_dir", "./data/uploads")?
        .set_default("search.url", "http://localhost:7700")?
        .set_default("search.api_key", "")?
        .set_default("limits.max_servers_per_user", 200)?
        .set_default("limits.max_channels_per_server", 500)?
        .set_default("limits.max_roles_per_server", 250)?
        .set_default("limits.max_members_per_server", 500_000)?
        .set_default("limits.max_message_length", 4000)?
        .set_default("limits.max_file_size_bytes", 104_857_600)? // 100MB default
        .set_default("limits.max_attachment_count", 10)?
        .set_default("scylla.nodes", "127.0.0.1:9042")?
        .set_default("scylla.keyspace", "nexus")?
        // Optional config file
        .add_source(config::File::with_name("config").required(false))
        // Environment variables (NEXUS_SERVER__HOST, NEXUS_DATABASE__URL, etc.)
        .add_source(
            config::Environment::with_prefix("NEXUS")
                .separator("__")
                .try_parsing(true),
        )
        .build()?;

    let app_config: AppConfig = cfg.try_deserialize()?;
    Ok(CONFIG.get_or_init(|| app_config))
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub scylla: ScyllaConfig,
    pub auth: AuthConfig,
    pub storage: StorageConfig,
    pub search: SearchConfig,
    pub limits: LimitsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// Public server name used for federation (e.g. "nexus.example.com").
    /// Maps to the `NEXUS__SERVER__NAME` env var or `server.name` in config.toml.
    pub name: String,
    pub host: String,
    pub port: u16,
    pub gateway_port: u16,
    pub voice_port: u16,
    /// Port used for server-to-server federation (default 8448).
    pub federation_port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    /// Redis connection URL — optional; omit for lite / in-process-only mode.
    pub url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScyllaConfig {
    /// ScyllaDB contact points — comma-separated, e.g. `127.0.0.1:9042,127.0.0.2:9042`
    pub nodes: String,
    /// Cassandra keyspace name
    pub keyspace: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    /// JWT signing secret (HS256) — should be 256+ bits of entropy
    pub jwt_secret: String,
    /// Access token TTL in seconds
    pub access_token_ttl_secs: u64,
    /// Refresh token TTL in seconds
    pub refresh_token_ttl_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StorageConfig {
    /// S3 endpoint URL (e.g., http://localhost:9000 for MinIO).
    /// Leave empty / unset in lite mode — files go to `data_dir` instead.
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    /// Local directory for file storage in lite mode (default: ./data/uploads).
    pub data_dir: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SearchConfig {
    /// MeiliSearch URL
    pub url: String,
    /// MeiliSearch API key
    pub api_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LimitsConfig {
    pub max_servers_per_user: u32,
    pub max_channels_per_server: u32,
    pub max_roles_per_server: u32,
    pub max_members_per_server: u32,
    pub max_message_length: u32,
    pub max_file_size_bytes: u64,
    pub max_attachment_count: u32,
}
