//! Snowflake ID generation.
//!
//! Like Discord, Nexus uses Snowflake IDs — globally unique, time-sortable,
//! generated without coordination. We use UUID v7 which provides the same
//! properties with broader ecosystem support.

use uuid::Uuid;

/// Generate a new Snowflake-style ID using UUID v7.
///
/// UUID v7 provides:
/// - Monotonically increasing (time-sortable)
/// - 48 bits of Unix timestamp (millisecond precision)
/// - 74 bits of randomness (guaranteed unique across nodes)
/// - Compatible with all UUID infrastructure (Postgres, etc.)
pub fn generate_id() -> Uuid {
    Uuid::now_v7()
}

/// Extract the approximate creation timestamp from a UUID v7.
pub fn extract_timestamp(id: Uuid) -> Option<chrono::DateTime<chrono::Utc>> {
    let bytes = id.as_bytes();
    // UUID v7: first 48 bits are millisecond timestamp
    let ms = ((bytes[0] as u64) << 40)
        | ((bytes[1] as u64) << 32)
        | ((bytes[2] as u64) << 24)
        | ((bytes[3] as u64) << 16)
        | ((bytes[4] as u64) << 8)
        | (bytes[5] as u64);

    chrono::DateTime::from_timestamp_millis(ms as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unique_ids() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_ids_are_time_sortable() {
        let id1 = generate_id();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id2 = generate_id();
        // UUID v7 IDs should sort chronologically
        assert!(id1 < id2);
    }

    #[test]
    fn test_extract_timestamp() {
        let before = chrono::Utc::now();
        let id = generate_id();
        let after = chrono::Utc::now();

        let extracted = extract_timestamp(id).expect("should extract timestamp");
        // Timestamp should be between before and after
        assert!(extracted >= before - chrono::Duration::milliseconds(1));
        assert!(extracted <= after + chrono::Duration::milliseconds(1));
    }
}
