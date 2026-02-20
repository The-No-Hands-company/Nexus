//! PostgreSQL/SQLite setup and connection helpers.

/// Health check — verify the database is reachable.
pub async fn health_check(pool: &sqlx::AnyPool) -> bool {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .is_ok()
}
