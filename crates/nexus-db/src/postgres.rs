//! PostgreSQL setup and connection helpers.

use sqlx::PgPool;

/// Health check — verify the database is reachable.
pub async fn health_check(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .is_ok()
}
