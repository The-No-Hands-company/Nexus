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
    /// A cross-server friend request sent to a user on this or another instance.
    #[serde(rename = "nexus.friend_request")]
    FriendRequest,
    /// A response (accept / deny) to a previously sent federated friend request.
    #[serde(rename = "nexus.friend_request.response")]
    FriendRequestResponse,
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

// ─── v0.8.5 Rich server identity ─────────────────────────────────────────────

/// Extended well-known response returned by Phase 8.5+ servers.
///
/// Published at `/.well-known/nexus/server` by servers that have filled in
/// instance metadata. Remote servers parse this when initiating peering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichWellKnownServer {
    /// Canonical server name for federation (required).
    #[serde(rename = "m.server")]
    pub server: String,
    /// Human-readable display name of this instance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Short description shown in directory / peering UX.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Admin contact — e-mail address or URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_contact: Option<String>,
    /// Software stack and version (e.g. `"Nexus 0.15.0"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub software_version: Option<String>,
    /// Approximate number of registered users on the instance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_count: Option<u64>,
    /// Number of servers this instance is currently peered with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_count: Option<u64>,
    /// Federation policy: `"open"` | `"allowlist"` | `"closed"`.
    #[serde(default = "default_federation_policy")]
    pub federation_policy: String,
}

fn default_federation_policy() -> String {
    "open".to_owned()
}

// ─── v0.8.5 Peer health & status ─────────────────────────────────────────────

/// Online / offline / degraded health status of a federated peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerStatus {
    /// Peer responded to the most recent ping within normal latency.
    Online,
    /// Peer did not respond to the most recent ping.
    Offline,
    /// Peer responded but with elevated latency or partial errors.
    Degraded,
    /// Peer has been manually blocked by an admin.
    Blocked,
}

/// Live health snapshot of a single federated peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerHealth {
    pub domain: String,
    pub status: PeerStatus,
    /// Most recent round-trip latency in milliseconds (None if offline).
    pub latency_ms: Option<u32>,
    /// ISO-8601 timestamp of the last successful contact.
    pub last_seen_at: Option<DateTime<Utc>>,
    /// Last error string, if any.
    pub last_error: Option<String>,
}

// ─── v0.8.5 Admin peer record ─────────────────────────────────────────────────

/// Full peer record as returned by `GET /api/v1/admin/federation/peers`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRecord {
    pub id: String,
    pub server_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub admin_contact: Option<String>,
    pub software_version: Option<String>,
    pub user_count: Option<i64>,
    pub trust_score: i16,
    pub federation_policy: String,
    pub is_blocked: bool,
    pub is_healthy: bool,
    pub latency_ms: Option<i32>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub first_seen_at: DateTime<Utc>,
    pub server_type: String,
}

// ─── v0.8.5 Peering request record ───────────────────────────────────────────

/// A pending or resolved peering request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRequest {
    pub id: String,
    /// `"outbound"` or `"inbound"`.
    pub direction: String,
    pub remote_domain: String,
    pub remote_display_name: Option<String>,
    pub remote_description: Option<String>,
    pub message: Option<String>,
    /// `"pending"` | `"accepted"` | `"rejected"` | `"cancelled"`.
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

// ─── v0.8.5 Admin federation status ──────────────────────────────────────────

/// Summary of this instance's federation state, returned by
/// `GET /api/v1/admin/federation/status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStatus {
    pub server_name: String,
    pub software_version: String,
    pub federation_enabled: bool,
    pub peer_count: u64,
    pub healthy_peer_count: u64,
    pub pending_inbound_requests: u64,
    pub pending_outbound_requests: u64,
    /// Total federated events processed since last restart.
    pub events_processed: u64,
    pub uptime_seconds: u64,
}

// ─── Federated friend request wire format ─────────────────────────────────────

/// Payload sent via `PUT /_nexus/federation/v1/friend_request`.
///
/// Sent by the **requesting** server to the **target** server when a user
/// wishes to befriend someone on another Nexus instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedFriendRequest {
    /// UUID of the requester on the **origin** server.
    pub requester_id: String,
    /// Username of the requester (no `@server` suffix).
    pub requester_username: String,
    /// Display name of the requester (optional).
    pub requester_display_name: Option<String>,
    /// Avatar key or URL of the requester (optional).
    pub requester_avatar: Option<String>,
    /// The origin server name (e.g. `nexus.example.com`).
    pub origin_server: String,
    /// The target user's local username on the **destination** server.
    pub target_username: String,
    /// ISO-8601 timestamp of the request.
    pub timestamp: DateTime<Utc>,
}

/// Payload sent via `PUT /_nexus/federation/v1/friend_request/respond`.
///
/// Sent by the **responding** server back to the **requesting** server when
/// the target user accepts or denies a federated friend request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedFriendResponse {
    /// UUID of the original requester (on the requesting server).
    pub requester_id: String,
    /// UUID of the responder (on the responding/destination server).
    pub responder_id: String,
    /// Username of the responder.
    pub responder_username: String,
    /// Display name of the responder (optional).
    pub responder_display_name: Option<String>,
    /// Avatar key or URL of the responder (optional).
    pub responder_avatar: Option<String>,
    /// The server that the responder belongs to.
    pub origin_server: String,
    /// `"accepted"` or `"denied"`.
    pub action: String,
    /// ISO-8601 timestamp.
    pub timestamp: DateTime<Utc>,
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
