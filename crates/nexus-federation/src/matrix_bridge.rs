//! Matrix Application Service (AS) bridge.
//!
//! The bridge acts as a Matrix AS registered on a Matrix homeserver
//! (Synapse, Conduit, etc.). It translates between:
//!
//! - **Nexus → Matrix**: when a Nexus user posts in a bridged channel the
//!   bridge relays the message to the Matrix room via the homeserver AS API.
//! - **Matrix → Nexus**: the homeserver pushes new Matrix events to this
//!   bridge via `PUT /_matrix/app/v1/transactions/{txnId}`. The bridge
//!   converts them to Nexus events and dispatches them internally.
//!
//! # Registration
//!
//! A `registration.yaml` file (not this crate) must be provided to the Matrix
//! homeserver that registers this AS with the correct `hs_token`, `as_token`,
//! and `url` fields.
//!
//! # Status: stub implementation
//!
//! This module provides the type definitions and handler stubs for the Matrix
//! AS protocol. Full relay logic will be implemented in v0.8.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

// ─── Types ───────────────────────────────────────────────────────────────────

/// A Matrix homeserver transaction pushed to the AS.
///
/// `PUT /_matrix/app/v1/transactions/{txnId}`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixTransaction {
    pub events: Vec<MatrixEvent>,
}

/// A stripped Matrix client event as received from the homeserver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub room_id: String,
    pub sender: String,
    pub origin_server_ts: i64,
    pub content: serde_json::Value,
    #[serde(default)]
    pub unsigned: serde_json::Value,
    pub event_id: Option<String>,
}

/// Payload for sending a message to a Matrix room via the CS API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixMessageContent {
    #[serde(rename = "msgtype")]
    pub msgtype: String,
    pub body: String,
    /// Formatted HTML body (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatted_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

// ─── Bridge configuration ─────────────────────────────────────────────────────

/// Configuration for the Matrix AS bridge.
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// The Matrix homeserver base URL (e.g. `https://matrix.org`).
    pub homeserver_url: String,
    /// Application service token (sent to HS in every AS request).
    pub as_token: String,
    /// Homeserver token (sent from HS to AS, used for auth validation).
    pub hs_token: String,
    /// `@bot:server.tld` — the ghost user used to relay Nexus messages into Matrix.
    pub bot_mxid: String,
}

// ─── Bridge ──────────────────────────────────────────────────────────────────

/// Matrix Application Service bridge.
///
/// Create one via [`MatrixBridge::new`] and call:
///
/// - [`MatrixBridge::handle_transaction`] from the AS HTTP handler.
/// - [`MatrixBridge::send_to_matrix`] when a Nexus message should be relayed.
pub struct MatrixBridge {
    config: BridgeConfig,
    http: reqwest::Client,
    /// Optional: channel → Matrix room ID mapping. Populated lazily.
    room_map: HashMap<String, String>,
}

impl MatrixBridge {
    /// Create a new bridge with the given configuration.
    pub fn new(config: BridgeConfig) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("Nexus-Federation/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build Matrix bridge http client");
        Self { config, http, room_map: HashMap::new() }
    }

    // ── Inbound (Matrix → Nexus) ────────────────────────────────────────────

    /// Handle an inbound transaction from the Matrix homeserver.
    ///
    /// Called from the `PUT /_matrix/app/v1/transactions/{txnId}` route.
    /// Returns a list of Nexus events to dispatch internally.
    pub async fn handle_transaction(&self, txn: MatrixTransaction) -> Vec<BridgedEvent> {
        let mut out = Vec::new();
        for ev in txn.events {
            match ev.event_type.as_str() {
                "m.room.message" => {
                    if let Some(bridged) = self.convert_matrix_message(&ev) {
                        out.push(bridged);
                    }
                }
                "m.room.member" => {
                    debug!("Matrix member event from {} in {}", ev.sender, ev.room_id);
                    // TODO: sync membership state to Nexus
                }
                other => {
                    debug!("Ignoring unrecognised Matrix event type: {}", other);
                }
            }
        }
        out
    }

    /// Convert a Matrix `m.room.message` to a [`BridgedEvent`].
    fn convert_matrix_message(&self, ev: &MatrixEvent) -> Option<BridgedEvent> {
        let body = ev.content.get("body")?.as_str()?.to_owned();
        let msgtype = ev.content.get("msgtype").and_then(|v| v.as_str()).unwrap_or("m.text");
        if msgtype != "m.text" && msgtype != "m.notice" {
            return None; // Skip files, stickers, etc. for now
        }
        Some(BridgedEvent::MessageCreate {
            matrix_room_id: ev.room_id.clone(),
            sender_mxid: ev.sender.clone(),
            body,
            timestamp_ms: ev.origin_server_ts,
        })
    }

    // ── Outbound (Nexus → Matrix) ───────────────────────────────────────────

    /// Relay a Nexus message to a Matrix room as the bridge bot.
    ///
    /// # Arguments
    ///
    /// * `room_id`      — The Matrix room ID (`!id:server_name`)
    /// * `display_name` — Nexus display name of the sender (for the message prefix)
    /// * `body`         — Plain-text message body
    pub async fn send_to_matrix(
        &self,
        room_id: &str,
        display_name: &str,
        body: &str,
    ) -> Result<(), BridgeError> {
        let txn_id = uuid::Uuid::new_v4().simple().to_string();
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            self.config.homeserver_url,
            urlencoded(room_id),
            txn_id
        );

        let content = MatrixMessageContent {
            msgtype: "m.text".to_owned(),
            // Format as `Display Name: message body` so Matrix users see attribution.
            body: format!("{}: {}", display_name, body),
            formatted_body: Some(format!(
                "<b>{}</b>: {}",
                html_escape(display_name),
                html_escape(body)
            )),
            format: Some("org.matrix.custom.html".to_owned()),
        };

        let resp = self
            .http
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.config.as_token))
            .json(&content)
            .send()
            .await
            .map_err(|e| BridgeError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(BridgeError::HomeserverError(status.as_u16(), body));
        }

        info!("Relayed message to Matrix room {}", room_id);
        Ok(())
    }

    // ── Room mapping ────────────────────────────────────────────────────────

    /// Map a Nexus channel ID to a Matrix room ID.
    ///
    /// Returns `None` if this channel has no Matrix bridge configured.
    pub fn matrix_room_for_channel(&self, channel_id: &str) -> Option<&str> {
        self.room_map.get(channel_id).map(String::as_str)
    }

    /// Register a channel ↔ room mapping.
    pub fn register_room_mapping(&mut self, channel_id: impl Into<String>, room_id: impl Into<String>) {
        self.room_map.insert(channel_id.into(), room_id.into());
    }
}

// ─── Bridged event ────────────────────────────────────────────────────────────

/// A normalised event produced by the bridge for Nexus to consume.
#[derive(Debug, Clone)]
pub enum BridgedEvent {
    MessageCreate {
        matrix_room_id: String,
        sender_mxid: String,
        body: String,
        timestamp_ms: i64,
    },
    MemberJoin {
        matrix_room_id: String,
        mxid: String,
    },
    MemberLeave {
        matrix_room_id: String,
        mxid: String,
    },
}

// ─── Bridge error ─────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Matrix homeserver returned {0}: {1}")]
    HomeserverError(u16, String),
    #[error("Room not found for channel '{0}'")]
    RoomNotFound(String),
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn urlencoded(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
