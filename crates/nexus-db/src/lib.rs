//! # nexus-db
//!
//! Database layer for Nexus.
//!
//! Supports two modes, selected automatically from the `DATABASE_URL`:
//!
//! * **Full mode** (`postgres://…`) — PostgreSQL + optional Redis + MinIO + MeiliSearch.
//! * **Lite mode** (`sqlite://…`) — embedded SQLite, no external services required.

pub mod any_compat;
pub mod postgres;
pub mod redis_pool;
pub mod repository;
pub mod search;
pub mod storage;

use anyhow::Result;

/// Which backing store is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbBackend {
    Postgres,
    Sqlite,
}

impl DbBackend {
    pub fn from_url(url: &str) -> Self {
        if url.starts_with("sqlite://") || url.starts_with("sqlite:") {
            DbBackend::Sqlite
        } else {
            DbBackend::Postgres
        }
    }
}

/// Shared database state passed through Axum extractors.
#[derive(Clone)]
pub struct Database {
    /// SQL pool — works with both Postgres and SQLite.
    pub pool: sqlx::AnyPool,
    /// Redis connection (`None` in lite mode or when `REDIS_URL` is unset).
    pub redis: Option<redis::aio::ConnectionManager>,
    /// Which backend is active.
    pub backend: DbBackend,
}

impl Database {
    /// Connect using the URL in `config.database.url`.
    pub async fn connect(config: &nexus_common::config::AppConfig) -> Result<Self> {
        // Register all built-in drivers (Postgres + SQLite).
        sqlx::any::install_default_drivers();

        let backend = DbBackend::from_url(&config.database.url);

        let pool = match backend {
            DbBackend::Postgres => {
                tracing::info!("Connecting to PostgreSQL…");
                sqlx::any::AnyPoolOptions::new()
                    .max_connections(config.database.max_connections)
                    .min_connections(config.database.min_connections)
                    .connect(&config.database.url)
                    .await?
            }
            DbBackend::Sqlite => {
                tracing::info!("Connecting to SQLite: {}", &config.database.url);
                sqlx::any::AnyPoolOptions::new()
                    .max_connections(1)
                    .min_connections(1)
                    .connect(&config.database.url)
                    .await?
            }
        };

        // Redis — optional in full mode, always skipped in lite mode.
        let redis = if backend == DbBackend::Postgres {
            match &config.redis.url {
                Some(url) => {
                    tracing::info!("Connecting to Redis…");
                    let client = redis::Client::open(url.as_str())?;
                    let mgr = redis::aio::ConnectionManager::new(client).await?;
                    tracing::info!("Connected to Redis");
                    Some(mgr)
                }
                None => {
                    tracing::info!("REDIS_URL not set — using in-process broadcast only");
                    None
                }
            }
        } else {
            None
        };

        Ok(Self { pool, redis, backend })
    }

    /// Run migrations appropriate for the active backend.
    pub async fn migrate(&self) -> Result<()> {
        tracing::info!("Running database migrations…");
        match self.backend {
            DbBackend::Postgres => {
                sqlx::migrate!("./migrations").run(&self.pool).await?;
            }
            DbBackend::Sqlite => {
                sqlx::migrate!("./migrations-lite").run(&self.pool).await?;
            }
        }
        tracing::info!("Migrations complete");
        Ok(())
    }
}
