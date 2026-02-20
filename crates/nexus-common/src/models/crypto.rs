//! E2EE / Signal Protocol domain models.
//!
//! These types represent the key-management layer that lives on the server.
//! The server is *deliberately* designed to be blind to plaintexts:
//!   - It stores only *public* key material
//!   - Ciphertext is stored as opaque blobs keyed by device UUID
//!   - Session state blobs are encrypted by the client before upload
//!
//! # Signal Protocol summary
//! ```text
//! Registration:
//!   client -> server: IdentityKey (Ed25519 public)
//!                     SignedPreKey (X25519 public + Ed25519 sig)
//!                     OneTimePreKeys (X25519 public × N)
//!
//! Key Exchange (X3DH):
//!   initiator fetches recipient's key bundle, derives shared secret locally
//!   server marks the consumed OneTimePreKey as used
//!
//! Messaging (Double Ratchet):
//!   client encrypts per-recipient using the derived session
//!   server stores { device_id -> ciphertext } map, never sees plaintext
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================
// Devices
// ============================================================

/// A registered device belonging to a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    /// Ed25519 public identity key, base64-encoded
    pub identity_key: String,
    /// X25519 signed pre-key (public), base64-encoded
    pub signed_pre_key: String,
    /// Ed25519 signature over `signed_pre_key`, base64-encoded
    pub signed_pre_key_sig: String,
    pub signed_pre_key_id: i32,
    pub device_type: DeviceType,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Desktop,
    Mobile,
    Browser,
    Unknown,
}

impl Default for DeviceType {
    fn default() -> Self {
        Self::Unknown
    }
}

// ============================================================
// Key Bundle (what initiators fetch to start a session)
// ============================================================

/// Full key bundle for one device — returned to X3DH initiators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBundle {
    pub device_id: Uuid,
    pub user_id: Uuid,
    pub identity_key: String,
    pub signed_pre_key: String,
    pub signed_pre_key_sig: String,
    pub signed_pre_key_id: i32,
    /// One-time pre-key — may be absent if the server ran out
    pub one_time_pre_key: Option<OtpkPublic>,
}

/// One-time pre-key public data (returned as part of a key bundle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtpkPublic {
    pub key_id: i32,
    pub public_key: String,
}

// ============================================================
// One-Time Pre-Keys
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneTimePreKey {
    pub id: Uuid,
    pub device_id: Uuid,
    pub key_id: i32,
    pub public_key: String,
    pub consumed: bool,
    pub created_at: DateTime<Utc>,
}

// ============================================================
// E2EE Sessions
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2eeSession {
    pub id: Uuid,
    pub owner_device_id: Uuid,
    pub remote_device_id: Uuid,
    /// Opaque encrypted blob; server stores but never reads
    pub session_state: String,
    pub ratchet_step: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================
// Encrypted Messages
// ============================================================

/// An encrypted message stored on the server.
///
/// `ciphertext_map` is a JSON object: `{ "<device_uuid>": { "type": 1, "body": "<base64>" } }`
/// where `type` 1 = PreKeySignalMessage (first message), 2 = SignalMessage (subsequent).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub sender_id: Uuid,
    pub sender_device_id: Uuid,
    /// `{ device_uuid: { type: u8, body: base64 } }` 
    pub ciphertext_map: serde_json::Value,
    pub attachment_meta: Option<serde_json::Value>,
    pub sequence: i64,
    pub client_ts: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// ============================================================
// E2EE Channel Config
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2eeChannel {
    pub channel_id: Uuid,
    pub enabled_by: Uuid,
    pub enabled_at: DateTime<Utc>,
    pub rotation_interval_secs: i32,
    pub last_rotated_at: DateTime<Utc>,
}

// ============================================================
// Device Verification
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceVerification {
    pub id: Uuid,
    pub verifier_id: Uuid,
    pub target_device_id: Uuid,
    pub method: VerificationMethod,
    pub verified_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethod {
    SafetyNumber,
    QrScan,
    Emoji,
}

// ============================================================
// API Request / Response shapes
// ============================================================

/// Register a new device and upload its initial key material.
#[derive(Debug, Deserialize)]
pub struct RegisterDeviceRequest {
    pub name: String,
    pub device_type: Option<DeviceType>,
    pub identity_key: String,
    pub signed_pre_key: String,
    pub signed_pre_key_sig: String,
    pub signed_pre_key_id: i32,
    /// Initial batch of one-time pre-keys (recommended: 100)
    pub one_time_pre_keys: Vec<OtpkUpload>,
}

/// A single one-time pre-key upload entry.
#[derive(Debug, Deserialize)]
pub struct OtpkUpload {
    pub key_id: i32,
    pub public_key: String,
}

/// Upload additional one-time pre-keys to replenish the server's stock.
#[derive(Debug, Deserialize)]
pub struct UploadOtpkRequest {
    pub keys: Vec<OtpkUpload>,
}

/// Upload a new signed pre-key rotation.
#[derive(Debug, Deserialize)]
pub struct RotateSignedPreKeyRequest {
    pub signed_pre_key: String,
    pub signed_pre_key_sig: String,
    pub signed_pre_key_id: i32,
}

/// Send an encrypted message.
#[derive(Debug, Deserialize)]
pub struct SendEncryptedMessageRequest {
    /// Map of device_uuid (string) → CiphertextEnvelope
    pub ciphertext_map: serde_json::Value,
    pub attachment_meta: Option<serde_json::Value>,
    /// Client-set timestamp (informational only)
    pub client_ts: Option<DateTime<Utc>>,
}

/// Update session state (ratchet advance).
#[derive(Debug, Deserialize)]
pub struct UpdateSessionRequest {
    pub session_state: String,
    pub ratchet_step: i32,
}

/// Enable E2EE on a channel.
#[derive(Debug, Deserialize)]
pub struct EnableE2eeRequest {
    pub rotation_interval_secs: Option<i32>,
}

/// Verify a device.
#[derive(Debug, Deserialize)]
pub struct VerifyDeviceRequest {
    pub method: VerificationMethod,
}

/// Response: how many one-time pre-keys remain for a device.
#[derive(Debug, Serialize)]
pub struct OtpkCountResponse {
    pub device_id: Uuid,
    pub remaining: i64,
}

/// Safety number — a human-verifiable fingerprint of two identity keys.
/// Computed client-side but the server provides the raw keys needed.
#[derive(Debug, Serialize)]
pub struct SafetyNumberResponse {
    pub local_identity_key: String,
    pub remote_identity_key: String,
    /// Pre-computed hex fingerprint (SHA-512 of sorted concat of both keys, truncated to 60 digits)
    pub fingerprint: String,
}
