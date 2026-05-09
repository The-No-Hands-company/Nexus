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
use sqlx::{Row, any::AnyRow};
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

pub fn get_opt_datetime(row: &AnyRow, col: &str) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    let s: Option<String> = row.try_get(col)?;
    s.map(|v| parse_datetime(&v).map_err(|e| sqlx::Error::Decode(e)))
        .transpose()
}

fn parse_datetime(
    s: &str,
) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Try RFC 3339 first (e.g. "2024-01-15T10:30:00+00:00")
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Normalize Postgres TIMESTAMPTZ::text output: "2026-02-22 13:04:47.779907+00"
    // Step 1: Replace first space with 'T' to approach ISO 8601.
    // Step 2: A short timezone suffix like "+00" has no minutes — append ":00".
    let iso = s.replacen(' ', "T", 1);
    let iso = {
        let len = iso.len();
        if len >= 3 {
            let last3 = &iso[len - 3..];
            if (last3.starts_with('+') || last3.starts_with('-'))
                && last3[1..].chars().all(|c| c.is_ascii_digit())
            {
                format!("{}:00", iso)
            } else {
                iso
            }
        } else {
            iso
        }
    };
    if let Ok(dt) = DateTime::parse_from_rfc3339(&iso) {
        return Ok(dt.with_timezone(&Utc));
    }
    // SQLite CURRENT_TIMESTAMP: "2024-01-15 10:30:00"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(dt.and_utc());
    }
    // SQLite with fractional seconds: "2024-01-15 10:30:00.123456"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    // ── parse_datetime ────────────────────────────────────────────────────────
    // parse_datetime is private, but we can reach it through get_datetime's
    // contract by testing the formats it must handle directly.

    fn parse(s: &str) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        parse_datetime(s)
    }

    #[test]
    fn rfc3339_with_utc_offset() {
        let dt = parse("2026-03-15T14:30:00+00:00").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 14);
    }

    #[test]
    fn rfc3339_with_positive_offset() {
        let dt = parse("2026-01-01T12:00:00+05:30").unwrap();
        // 12:00 +05:30 = 06:30 UTC
        assert_eq!(dt.hour(), 6);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn postgres_timestamptz_text_format() {
        // Postgres TEXT output: space separator, short +00 suffix
        let dt = parse("2026-02-22 13:04:47.779907+00").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 2);
        assert_eq!(dt.day(), 22);
        assert_eq!(dt.hour(), 13);
    }

    #[test]
    fn sqlite_current_timestamp_format() {
        // SQLite CURRENT_TIMESTAMP: no timezone, space separator
        let dt = parse("2024-01-15 10:30:00").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn sqlite_with_fractional_seconds() {
        let dt = parse("2024-06-20 08:15:33.123456").unwrap();
        assert_eq!(dt.hour(), 8);
        assert_eq!(dt.minute(), 15);
        assert_eq!(dt.second(), 33);
    }

    #[test]
    fn invalid_string_returns_error() {
        assert!(parse("not a timestamp").is_err());
        assert!(parse("").is_err());
        assert!(parse("2024-13-45").is_err()); // impossible month/day
    }

    #[test]
    fn result_is_always_utc() {
        // Regardless of input offset, output must be UTC
        for s in &[
            "2026-01-01T00:00:00+05:00",
            "2026-01-01 00:00:00",
            "2026-01-01T00:00:00+00:00",
        ] {
            let dt = parse(s).unwrap();
            assert_eq!(dt.timezone(), chrono::Utc, "not UTC for input: {s}");
        }
    }
}
