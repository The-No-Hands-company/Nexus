//! Redis connection pool and helpers.

use redis::aio::ConnectionManager;
use redis::AsyncCommands;

/// Set a key with expiration (for sessions, rate limits, etc.).
pub async fn set_ex(
    conn: &mut ConnectionManager,
    key: &str,
    value: &str,
    ttl_secs: u64,
) -> Result<(), redis::RedisError> {
    conn.set_ex(key, value, ttl_secs).await
}

/// Get a value by key.
pub async fn get(conn: &mut ConnectionManager, key: &str) -> Result<Option<String>, redis::RedisError> {
    conn.get(key).await
}

/// Delete a key.
pub async fn del(conn: &mut ConnectionManager, key: &str) -> Result<(), redis::RedisError> {
    conn.del(key).await
}

/// Check if a key exists.
pub async fn exists(conn: &mut ConnectionManager, key: &str) -> Result<bool, redis::RedisError> {
    conn.exists(key).await
}

/// Publish to a Redis channel (for real-time event distribution).
pub async fn publish(
    conn: &mut ConnectionManager,
    channel: &str,
    message: &str,
) -> Result<(), redis::RedisError> {
    conn.publish(channel, message).await
}

/// Increment a counter (for rate limiting).
pub async fn incr_expire(
    conn: &mut ConnectionManager,
    key: &str,
    ttl_secs: u64,
) -> Result<i64, redis::RedisError> {
    let count: i64 = conn.incr(key, 1).await?;
    if count == 1 {
        let _: () = conn.expire(key, ttl_secs as i64).await?;
    }
    Ok(count)
}
