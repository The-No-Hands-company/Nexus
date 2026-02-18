//! Federation data types — event shapes, transaction envelopes, and server info.
//!
//! These types model the wire format used in server-to-server communication,
//! inspired by Matrix's federation PDU/EDU structure but adapted for Nexus.

use base64::Engine as _;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ─── Server info ─────────────────────────────────────────────────────────────

/// Metadata published at `/.well-known/nexus/server` and
/// `/_nexus/key/v2/server`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// The canonical server name (e.g. `nexus.example.com`).
    pub server_name: String,
    /// Server software name and version.
    pub server_version: String,
    /// Current public signing keys, keyed by key ID.
    pub verify_keys: HashMap<String, VerifyKey>,
    /// Timestamp this key document was generated.
    pub valid_until_ts: DateTime<Utc>,
}

/// A single public verify key entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyKey {
    /// Base64url-encoded Ed25519 public key bytes.
    pub key: String,
}

// ─── Federated events ────────────────────────────────────────────────────────

/// A persistent federation event (PDU — Persistent Data Unit).
///
/// Nexus PDUs follow a simplified Matrix PDU shape to maintain compatibility
/// where possible with Matrix client-server and federation parsers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationEvent {
    /// Globally unique event ID (`$<base64url>:<server_name>`).
    pub event_id: String,
    /// Origin server name.
    pub origin: String,
    /// Destination server name (if targeted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    /// Event type (e.g. `nexus.message.create`, `nexus.member.join`).
    #[serde(rename = "type")]
    pub event_type: FederationEventType,
    /// The room / channel this event belongs to.
    pub room_id: String,
    /// User who sent the event (`@user:server_name`).
    pub sender: String,
    /// Unix millisecond timestamp.
    pub origin_server_ts: i64,
    /// Structured event content — varies per event type.
    pub content: serde_json::Value,
    /// Previous event IDs for DAG ordering (simplified to single prev for now).
    pub prev_events: Vec<String>,
    /// Ed25519 signatures from the origin server.
    pub signatures: HashMap<String, HashMap<String, String>>,
    /// Hash of the event content for integrity verification.
    pub hashes: EventHashes,
}

/// Event integrity hashes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHashes {
    /// SHA-256 of canonical JSON, base64url-encoded.
    pub sha256: String,
}

/// Typed event kinds understood by the Nexus federation layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FederationEventType {
    /// A new message was created.
    #[serde(rename = "nexus.message.create")]
    MessageCreate,
    /// A message was edited.
    #[serde(rename = "nexus.message.update")]
    MessageUpdate,
    /// A message was deleted.
    #[serde(rename = "nexus.message.delete")]
    MessageDelete,
    /// A user joined a federated room.
    #[serde(rename = "nexus.member.join")]
    MemberJoin,
    /// A user left or was kicked from a federated room.
    #[serde(rename = "nexus.member.leave")]
    MemberLeave,
    /// Room / channel state was updated.
    #[serde(rename = "nexus.room.state")]
    RoomState,
    /// Typing indicator (ephemeral, not persisted).
    #[serde(rename = "nexus.typing")]
    Typing,
    /// Presence update (ephemeral).
    #[serde(rename = "nexus.presence")]
    Presence,
    /// Bridge relay from an external network (Matrix, Discord).
    #[serde(rename = "nexus.bridge.relay")]
    BridgeRelay,
    /// Catch-all for unknown event types.
    #[serde(untagged)]
    Unknown(String),
}

// ─── Transaction envelope ────────────────────────────────────────────────────

/// A federation transaction — the envelope sent via `PUT /send/{txnId}`.
///
/// A transaction batches multiple PDUs and EDUs together to reduce the number
/// of HTTP round-trips between servers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationTransaction {
    /// Originating server name.
    pub origin: String,
    /// Destination server name.
    pub destination: String,
    /// Unix millisecond timestamp on the origin server.
    pub origin_server_ts: i64,
    /// Persistent Data Units (stored events).
    #[serde(default)]
    pub pdus: Vec<FederationEvent>,
    /// Ephemeral Data Units (typing, presence — not stored).
    #[serde(default)]
    pub edus: Vec<serde_json::Value>,
}

impl FederationTransaction {
    /// Create a new empty transaction ready to be populated.
    pub fn new(origin: impl Into<String>, destination: impl Into<String>) -> Self {
        Self {
            origin: origin.into(),
            destination: destination.into(),
            origin_server_ts: Utc::now().timestamp_millis(),
            pdus: Vec::new(),
            edus: Vec::new(),
        }
    }
}

// ─── Join protocol ────────────────────────────────────────────────────────────

/// Payload returned by `GET /make_join/{roomId}/{userId}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MakeJoinResponse {
    pub room_version: String,
    /// Template join event the client should fill in and sign.
    pub event: serde_json::Value,
}

/// Payload returned by `PUT /send_join/{roomId}/{eventId}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendJoinResponse {
    /// Current room state snapshot at the join point.
    pub state: Vec<FederationEvent>,
    /// Auth chain (events needed to validate the state).
    pub auth_chain: Vec<FederationEvent>,
}

// ─── Public directory listing ─────────────────────────────────────────────────

/// A single server entry in the public directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryServer {
    pub server_name: String,
    pub description: Option<String>,
    /// Number of public rooms available for federation.
    pub public_room_count: u64,
    /// Approximate total member count across public rooms.
    pub total_users: u64,
    /// Icon/avatar URL for the server.
    pub icon_url: Option<String>,
}

/// A single public-room entry returned by the directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryRoom {
    /// Fully-qualified room ID (`!id:server_name`).
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub member_count: u64,
    pub server_name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Invite-only vs open join.
    pub join_rule: JoinRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum JoinRule {
    #[default]
    Public,
    Invite,
    Knock,
}

/// Paginated directory listing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryListingResponse {
    pub rooms: Vec<DirectoryRoom>,
    pub total_count: u64,
    pub next_batch: Option<String>,
}

// ─── Well-known response ──────────────────────────────────────────────────────

/// Response shape for `/.well-known/nexus/server`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WellKnownServer {
    /// The delegated server name (may differ from the queried hostname for delegation).
    #[serde(rename = "m.server")]
    pub server: String,
}

// ─── Utilities ────────────────────────────────────────────────────────────────

/// Generate a new locally-unique event ID on this server.
pub fn new_event_id(server_name: &str) -> String {
    let id = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Uuid::new_v4().as_bytes());
    format!("${}:{}", id, server_name)
}

/// Build a Matrix-style MXID from a local user and server.
///
/// Example: `@alice:nexus.example.com`
pub fn mxid(local_part: &str, server_name: &str) -> String {
    format!("@{}:{}", local_part, server_name)
}

/// Build a federated room ID from a local channel ID and server.
///
/// Example: `!channelid:nexus.example.com`
pub fn room_id(channel_id: &str, server_name: &str) -> String {
    format!("!{}:{}", channel_id, server_name)
}
