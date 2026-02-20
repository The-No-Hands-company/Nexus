//! Key-store repository — CRUD for E2EE key material.
//!
//! All functions work against PostgreSQL via sqlx non-macro queries.
//! The server is *write-once* for identity keys (registration) and
//! *consume-once* for one-time pre-keys (X3DH exchange).

use anyhow::Result;
use nexus_common::models::crypto::{
    Device, DeviceVerification, E2eeChannel, E2eeSession, EncryptedMessage, KeyBundle, OneTimePreKey,
    OtpkPublic,
};

use uuid::Uuid;

// ============================================================
// Devices
// ============================================================

/// Register a new device for a user.
#[allow(clippy::too_many_arguments)]
pub async fn create_device(
    pool: &sqlx::AnyPool,
    user_id: Uuid,
    name: &str,
    device_type: &str,
    identity_key: &str,
    signed_pre_key: &str,
    signed_pre_key_sig: &str,
    signed_pre_key_id: i32,
) -> Result<Device> {
    let row = sqlx::query_as::<_, Device>(
        r#"
        INSERT INTO devices
            (user_id, name, device_type, identity_key,
             signed_pre_key, signed_pre_key_sig, signed_pre_key_id)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(user_id.to_string())
    .bind(name)
    .bind(device_type)
    .bind(identity_key)
    .bind(signed_pre_key)
    .bind(signed_pre_key_sig)
    .bind(signed_pre_key_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// List all devices for a user (public info only — no secret material stored server-side).
pub async fn list_devices(pool: &sqlx::AnyPool, user_id: Uuid) -> Result<Vec<Device>> {
    let rows = sqlx::query_as::<_, Device>(
        "SELECT * FROM devices WHERE user_id = ? ORDER BY created_at ASC",
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Find a single device by ID.
pub async fn find_device(pool: &sqlx::AnyPool, device_id: Uuid) -> Result<Option<Device>> {
    let row = sqlx::query_as::<_, Device>("SELECT * FROM devices WHERE id = ?")
        .bind(device_id.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

/// Update the signed pre-key (rotation).
pub async fn rotate_signed_pre_key(
    pool: &sqlx::AnyPool,
    device_id: Uuid,
    signed_pre_key: &str,
    signed_pre_key_sig: &str,
    signed_pre_key_id: i32,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE devices
        SET signed_pre_key = ?,
            signed_pre_key_sig = ?,
            signed_pre_key_id = ?,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(signed_pre_key)
    .bind(signed_pre_key_sig)
    .bind(signed_pre_key_id)
    .bind(device_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

/// Touch last_seen_at for a device.
pub async fn touch_device(pool: &sqlx::AnyPool, device_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE devices SET last_seen_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(device_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete a device and all associated key material.
pub async fn delete_device(pool: &sqlx::AnyPool, device_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM devices WHERE id = ?")
        .bind(device_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

// ============================================================
// One-Time Pre-Keys
// ============================================================

/// Bulk-insert one-time pre-keys for a device.
pub async fn insert_one_time_pre_keys(
    pool: &sqlx::AnyPool,
    device_id: Uuid,
    keys: &[(i32, String)],
) -> Result<usize> {
    let mut inserted = 0usize;
    for (key_id, public_key) in keys {
        sqlx::query(
            r#"
            INSERT INTO one_time_pre_keys (device_id, key_id, public_key)
            VALUES (?, ?, ?)
            ON CONFLICT (device_id, key_id) DO NOTHING
            "#,
        )
        .bind(device_id.to_string())
        .bind(key_id)
        .bind(public_key)
        .execute(pool)
        .await?;
        inserted += 1;
    }
    Ok(inserted)
}

/// Consume one one-time pre-key for a device (atomically marks it used and returns it).
/// Returns `None` if the device has run out of one-time pre-keys.
pub async fn consume_one_time_pre_key(
    pool: &sqlx::AnyPool,
    device_id: Uuid,
) -> Result<Option<OtpkPublic>> {
    #[derive(sqlx::FromRow)]
    struct OtpkRow {
        key_id: i32,
        public_key: String,
    }

    let row = sqlx::query_as::<_, OtpkRow>(
        r#"
        UPDATE one_time_pre_keys
        SET consumed = true
        WHERE id = (
            SELECT id FROM one_time_pre_keys
            WHERE device_id = ? AND NOT consumed
            ORDER BY key_id ASC
            LIMIT 1
        )
        RETURNING key_id, public_key
        "#,
    )
    .bind(device_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| OtpkPublic {
        key_id: r.key_id,
        public_key: r.public_key,
    }))
}

/// Count remaining (unconsumed) one-time pre-keys for a device.
pub async fn count_one_time_pre_keys(pool: &sqlx::AnyPool, device_id: Uuid) -> Result<i64> {
    #[derive(sqlx::FromRow)]
    struct CountRow {
        count: i64,
    }
    let row = sqlx::query_as::<_, CountRow>(
        "SELECT COUNT(*) AS count FROM one_time_pre_keys WHERE device_id = ? AND NOT consumed",
    )
    .bind(device_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(row.count)
}

// ============================================================
// Key Bundle
// ============================================================

/// Fetch a full key bundle for a device (for X3DH initiators).
/// Atomically consumes one OTPk.
pub async fn get_key_bundle(pool: &sqlx::AnyPool, device_id: Uuid) -> Result<Option<KeyBundle>> {
    let device = match find_device(pool, device_id).await? {
        Some(d) => d,
        None => return Ok(None),
    };
    let otpk = consume_one_time_pre_key(pool, device_id).await?;
    Ok(Some(KeyBundle {
        device_id: device.id,
        user_id: device.user_id,
        identity_key: device.identity_key,
        signed_pre_key: device.signed_pre_key,
        signed_pre_key_sig: device.signed_pre_key_sig,
        signed_pre_key_id: device.signed_pre_key_id,
        one_time_pre_key: otpk,
    }))
}

/// Fetch key bundles for ALL devices of a user (multi-device send).
pub async fn get_all_key_bundles(pool: &sqlx::AnyPool, user_id: Uuid) -> Result<Vec<KeyBundle>> {
    let devices = list_devices(pool, user_id).await?;
    let mut bundles = Vec::with_capacity(devices.len());
    for device in devices {
        let otpk = consume_one_time_pre_key(pool, device.id).await?;
        bundles.push(KeyBundle {
            device_id: device.id,
            user_id: device.user_id,
            identity_key: device.identity_key,
            signed_pre_key: device.signed_pre_key,
            signed_pre_key_sig: device.signed_pre_key_sig,
            signed_pre_key_id: device.signed_pre_key_id,
            one_time_pre_key: otpk,
        });
    }
    Ok(bundles)
}

// ============================================================
// E2EE Sessions
// ============================================================

/// Upsert a session state blob (client ratchets and re-uploads).
pub async fn upsert_session(
    pool: &sqlx::AnyPool,
    owner_device_id: Uuid,
    remote_device_id: Uuid,
    session_state: &str,
    ratchet_step: i32,
) -> Result<E2eeSession> {
    let row = sqlx::query_as::<_, E2eeSession>(
        r#"
        INSERT INTO e2ee_sessions
            (owner_device_id, remote_device_id, session_state, ratchet_step)
        VALUES (?, ?, ?, ?)
        ON CONFLICT (owner_device_id, remote_device_id) DO UPDATE
            SET session_state = EXCLUDED.session_state,
                ratchet_step  = EXCLUDED.ratchet_step,
                updated_at    = CURRENT_TIMESTAMP
        RETURNING *
        "#,
    )
    .bind(owner_device_id.to_string())
    .bind(remote_device_id.to_string())
    .bind(session_state)
    .bind(ratchet_step)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Fetch session state for a device pair.
pub async fn get_session(
    pool: &sqlx::AnyPool,
    owner_device_id: Uuid,
    remote_device_id: Uuid,
) -> Result<Option<E2eeSession>> {
    let row = sqlx::query_as::<_, E2eeSession>(
        "SELECT * FROM e2ee_sessions WHERE owner_device_id = ? AND remote_device_id = ?",
    )
    .bind(owner_device_id.to_string())
    .bind(remote_device_id.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

// ============================================================
// Encrypted Messages
// ============================================================

/// Store an encrypted message.
pub async fn store_encrypted_message(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
    sender_id: Uuid,
    sender_device_id: Uuid,
    ciphertext_map: &serde_json::Value,
    attachment_meta: Option<&serde_json::Value>,
    client_ts: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<EncryptedMessage> {
    let row = sqlx::query_as::<_, EncryptedMessage>(
        r#"
        INSERT INTO encrypted_messages
            (channel_id, sender_id, sender_device_id, ciphertext_map,
             attachment_meta, sequence, client_ts)
        VALUES (
            ?, ?, ?, ?, ?,
            COALESCE(
                (SELECT MAX(sequence) + 1 FROM encrypted_messages WHERE channel_id = ?),
                1
            ),
            ?
        )
        RETURNING *
        "#,
    )
    .bind(channel_id.to_string())
    .bind(sender_id.to_string())
    .bind(sender_device_id.to_string())
    .bind(serde_json::to_string(ciphertext_map).unwrap_or_default())
    .bind(attachment_meta.map(|v| serde_json::to_string(v).unwrap_or_default()))
    .bind(channel_id.to_string())
    .bind(client_ts.map(|d| d.to_rfc3339()))
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// List encrypted messages for a channel, paginated, newest-first.
pub async fn list_encrypted_messages(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
    before_sequence: Option<i64>,
    limit: i64,
) -> Result<Vec<EncryptedMessage>> {
    let rows = if let Some(before) = before_sequence {
        sqlx::query_as::<_, EncryptedMessage>(
            r#"
            SELECT * FROM encrypted_messages
            WHERE channel_id = ? AND sequence < ?
            ORDER BY sequence DESC
            LIMIT ?
            "#,
        )
        .bind(channel_id.to_string())
        .bind(before)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, EncryptedMessage>(
            r#"
            SELECT * FROM encrypted_messages
            WHERE channel_id = ?
            ORDER BY sequence DESC
            LIMIT ?
            "#,
        )
        .bind(channel_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    Ok(rows)
}

// ============================================================
// E2EE Channels
// ============================================================

/// Mark a channel as E2EE.
pub async fn enable_e2ee_channel(
    pool: &sqlx::AnyPool,
    channel_id: Uuid,
    enabled_by: Uuid,
    rotation_interval_secs: i32,
) -> Result<E2eeChannel> {
    let row = sqlx::query_as::<_, E2eeChannel>(
        r#"
        INSERT INTO e2ee_channels (channel_id, enabled_by, rotation_interval_secs)
        VALUES (?, ?, ?)
        ON CONFLICT (channel_id) DO UPDATE
            SET rotation_interval_secs = EXCLUDED.rotation_interval_secs,
                last_rotated_at        = CURRENT_TIMESTAMP
        RETURNING *
        "#,
    )
    .bind(channel_id.to_string())
    .bind(enabled_by.to_string())
    .bind(rotation_interval_secs)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Get E2EE config for a channel (returns None if not E2EE).
pub async fn get_e2ee_channel(pool: &sqlx::AnyPool, channel_id: Uuid) -> Result<Option<E2eeChannel>> {
    let row = sqlx::query_as::<_, E2eeChannel>(
        "SELECT * FROM e2ee_channels WHERE channel_id = ?",
    )
    .bind(channel_id.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Record a key rotation event.
pub async fn record_key_rotation(pool: &sqlx::AnyPool, channel_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE e2ee_channels SET last_rotated_at = CURRENT_TIMESTAMP WHERE channel_id = ?",
    )
    .bind(channel_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

// ============================================================
// Device Verification
// ============================================================

/// Record that a user has verified a device.
pub async fn verify_device(
    pool: &sqlx::AnyPool,
    verifier_id: Uuid,
    target_device_id: Uuid,
    method: &str,
) -> Result<DeviceVerification> {
    let row = sqlx::query_as::<_, DeviceVerification>(
        r#"
        INSERT INTO device_verifications (verifier_id, target_device_id, method)
        VALUES (?, ?, ?)
        ON CONFLICT (verifier_id, target_device_id) DO UPDATE
            SET method = EXCLUDED.method,
                verified_at = CURRENT_TIMESTAMP
        RETURNING *
        "#,
    )
    .bind(verifier_id.to_string())
    .bind(target_device_id.to_string())
    .bind(method)
    .fetch_one(pool)
    .await?;

    // Also mark the device itself as verified
    sqlx::query("UPDATE devices SET verified = true WHERE id = ?")
        .bind(target_device_id.to_string())
        .execute(pool)
        .await?;

    Ok(row)
}

/// Check whether a verifier has verified a target device.
pub async fn is_device_verified(
    pool: &sqlx::AnyPool,
    verifier_id: Uuid,
    target_device_id: Uuid,
) -> Result<bool> {
    #[derive(sqlx::FromRow)]
    struct ExistsRow {
        exists: i64,
    }
    let row = sqlx::query_as::<_, ExistsRow>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM device_verifications
            WHERE verifier_id = ? AND target_device_id = ?
        ) AS exists
        "#,
    )
    .bind(verifier_id.to_string())
    .bind(target_device_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(row.exists != 0)
}

/// List all verifications made by a user.
pub async fn list_verifications(
    pool: &sqlx::AnyPool,
    verifier_id: Uuid,
) -> Result<Vec<DeviceVerification>> {
    let rows = sqlx::query_as::<_, DeviceVerification>(
        "SELECT * FROM device_verifications WHERE verifier_id = ? ORDER BY verified_at DESC",
    )
    .bind(verifier_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Fetch the one-time pre-key for a device by key_id (for debugging / admin purposes).
pub async fn get_one_time_pre_key(
    pool: &sqlx::AnyPool,
    device_id: Uuid,
    key_id: i32,
) -> Result<Option<OneTimePreKey>> {
    let row = sqlx::query_as::<_, OneTimePreKey>(
        "SELECT * FROM one_time_pre_keys WHERE device_id = ? AND key_id = ?",
    )
    .bind(device_id.to_string())
    .bind(key_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
