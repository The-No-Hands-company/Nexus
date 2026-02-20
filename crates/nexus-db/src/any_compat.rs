//! AnyPool row-extraction helpers.
//!
//! `sqlx::AnyPool` only natively decodes primitive types (i8/i16/i32/i64,
//! f32/f64, bool, String, &[u8]).  Every column that stores a UUID, an ISO
//! timestamp, a JSON blob, or a `Vec<Uuid>` must be fetched as `String` and
//! converted here.
//!
//! All functions return `sqlx::Error` so they fit naturally into
//! `sqlx::FromRow` manual implementations.

use chrono::{DateTime, Utc};
use sqlx::{any::AnyRow, Row};
use uuid::Uuid;

// ── Uuid ─────────────────────────────────────────────────────────────────────

pub fn get_uuid(row: &AnyRow, col: &str) -> Result<Uuid, sqlx::Error> {
    let s: String = row.try_get(col)?;
    Uuid::parse_str(&s).map_err(|e| sqlx::Error::Decode(Box::new(e) as _))
}

pub fn get_opt_uuid(row: &AnyRow, col: &str) -> Result<Option<Uuid>, sqlx::Error> {
    let s: Option<String> = row.try_get(col)?;
    s.map(|v| Uuid::parse_str(&v).map_err(|e| sqlx::Error::Decode(Box::new(e) as _)))
        .transpose()
}

// ── DateTime<Utc> ─────────────────────────────────────────────────────────────

pub fn get_datetime(row: &AnyRow, col: &str) -> Result<DateTime<Utc>, sqlx::Error> {
    let s: String = row.try_get(col)?;
    // SQLite stores CURRENT_TIMESTAMP as "YYYY-MM-DD HH:MM:SS"
    // Postgres (via Any text protocol) sends ISO 8601 / RFC 3339
    parse_datetime(&s).map_err(|e| sqlx::Error::Decode(e))
}

pub fn get_opt_datetime(
    row: &AnyRow,
    col: &str,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    let s: Option<String> = row.try_get(col)?;
    s.map(|v| parse_datetime(&v).map_err(|e| sqlx::Error::Decode(e)))
        .transpose()
}

fn parse_datetime(
    s: &str,
) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Try RFC 3339 first (Postgres output: "2024-01-15T10:30:00+00:00")
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Try SQLite CURRENT_TIMESTAMP format: "2024-01-15 10:30:00"
    if let Ok(dt) =
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
    {
        return Ok(dt.and_utc());
    }
    // Try with fractional seconds: "2024-01-15 10:30:00.123456"
    if let Ok(dt) =
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")
    {
        return Ok(dt.and_utc());
    }
    Err(format!("cannot parse timestamp: {s}").into())
}

// ── serde_json::Value ─────────────────────────────────────────────────────────

pub fn get_json_value(row: &AnyRow, col: &str) -> Result<serde_json::Value, sqlx::Error> {
    let s: String = row.try_get(col)?;
    serde_json::from_str(&s).map_err(|e| sqlx::Error::Decode(Box::new(e) as _))
}

pub fn get_opt_json_value(
    row: &AnyRow,
    col: &str,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    let s: Option<String> = row.try_get(col)?;
    s.map(|v| serde_json::from_str(&v).map_err(|e| sqlx::Error::Decode(Box::new(e) as _)))
        .transpose()
}

// ── Vec<Uuid> ─────────────────────────────────────────────────────────────────

/// Decode a JSON-array-of-strings column (e.g. `["uuid1","uuid2"]`) → Vec<Uuid>
pub fn get_uuid_vec(row: &AnyRow, col: &str) -> Result<Vec<Uuid>, sqlx::Error> {
    let s: String = row.try_get(col)?;
    if s == "[]" || s.is_empty() {
        return Ok(vec![]);
    }
    let strs: Vec<String> =
        serde_json::from_str(&s).map_err(|e| sqlx::Error::Decode(Box::new(e) as _))?;
    strs.iter()
        .map(|v| Uuid::parse_str(v).map_err(|e| sqlx::Error::Decode(Box::new(e) as _)))
        .collect()
}

/// Decode a JSON-array-of-strings column → Vec<String>
pub fn get_string_vec(row: &AnyRow, col: &str) -> Result<Vec<String>, sqlx::Error> {
    let s: String = row.try_get(col)?;
    if s == "[]" || s.is_empty() {
        return Ok(vec![]);
    }
    serde_json::from_str(&s).map_err(|e| sqlx::Error::Decode(Box::new(e) as _))
}
