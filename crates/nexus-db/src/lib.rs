//! # nexus-db
//!
//! Database layer for Nexus. Manages connections to:
//! - **PostgreSQL** — Users, servers, channels, roles, members, invites (relational data)
//! - **ScyllaDB** — Messages (write-heavy, time-series, partitioned by channel)
//! - **Redis** — Sessions, presence, rate limiting, pub/sub event distribution

pub mod postgres;
pub mod redis_pool;
pub mod repository;

use anyhow::Result;
use sqlx::PgPool;

/// Shared database state passed through Axum extractors.
#[derive(Clone)]
pub struct Database {
    pub pg: PgPool,
    pub redis: redis::aio::ConnectionManager,
}

impl Database {
    /// Connect to all database backends.
    pub async fn connect(config: &nexus_common::config::AppConfig) -> Result<Self> {
        tracing::info!("Connecting to PostgreSQL...");
        let pg = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.database.max_connections)
            .min_connections(config.database.min_connections)
            .connect(&config.database.url)
            .await?;

        tracing::info!("Connected to PostgreSQL");

        tracing::info!("Connecting to Redis...");
        let redis_client = redis::Client::open(config.redis.url.as_str())?;
        let redis = redis::aio::ConnectionManager::new(redis_client).await?;
        tracing::info!("Connected to Redis");

        Ok(Self { pg, redis })
    }

    /// Run database migrations.
    pub async fn migrate(&self) -> Result<()> {
        tracing::info!("Running database migrations...");
        sqlx::migrate!("./migrations").run(&self.pg).await?;
        tracing::info!("Migrations complete");
        Ok(())
    }
}
