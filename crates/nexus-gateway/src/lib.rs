//! # nexus-gateway
//!
//! Real-time WebSocket gateway for Nexus. Handles:
//! - Client connections with authentication
//! - Event dispatch (messages, presence, typing, etc.)
//! - Heartbeat/keepalive
//! - Session resume on reconnect
//!
//! Protocol inspired by Discord's Gateway but cleaner:
//! - Opcodes are named, not numbered
//! - Events are typed and documented
//! - No hidden rate limits

pub mod events;
pub mod session;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Gateway state.
#[derive(Clone)]
pub struct GatewayState {
    /// Broadcast channel for dispatching events to all connected clients.
    /// In production, this would use Redis pub/sub for multi-node support.
    pub broadcast: broadcast::Sender<GatewayEvent>,
    pub db: nexus_db::Database,
}

impl GatewayState {
    pub fn new(db: nexus_db::Database) -> Self {
        let (broadcast, _) = broadcast::channel(10_000);
        Self { broadcast, db }
    }
}

/// Gateway opcodes — what the client and server send to each other.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", content = "d")]
pub enum GatewayMessage {
    /// Client → Server: Authenticate with access token
    Identify { token: String },

    /// Server → Client: Connection accepted, here's your session info
    Ready {
        session_id: String,
        user: serde_json::Value,
        servers: Vec<serde_json::Value>,
    },

    /// Bidirectional: Keepalive ping/pong
    Heartbeat { timestamp: i64 },

    /// Server → Client: Heartbeat acknowledged
    HeartbeatAck { timestamp: i64 },

    /// Client → Server: Resume a disconnected session
    Resume {
        session_id: String,
        token: String,
        sequence: u64,
    },

    /// Server → Client: An event occurred
    Dispatch {
        event: String,
        data: serde_json::Value,
        sequence: u64,
    },

    /// Server → Client: Reconnect requested (server restarting, etc.)
    Reconnect,

    /// Server → Client: Session invalidated, must re-identify
    InvalidSession,

    /// Client → Server: Request presence update
    PresenceUpdate {
        status: String,
        custom_status: Option<String>,
    },

    /// Client → Server: Typing indicator
    TypingStart { channel_id: String },

    /// Client → Server: Join voice channel
    VoiceStateUpdate {
        server_id: Option<String>,
        channel_id: Option<String>,
        self_mute: bool,
        self_deaf: bool,
    },
}

/// Events broadcast to connected clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayEvent {
    pub event_type: String,
    pub data: serde_json::Value,
    /// Which server this event belongs to (for filtering)
    pub server_id: Option<uuid::Uuid>,
    /// Which channel this event belongs to
    pub channel_id: Option<uuid::Uuid>,
    /// Which user triggered this event
    pub user_id: Option<uuid::Uuid>,
}

/// Build the gateway WebSocket router.
pub fn build_router(state: GatewayState) -> Router {
    Router::new()
        .route("/gateway", get(ws_handler))
        .with_state(Arc::new(state))
}

/// WebSocket upgrade handler.
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<GatewayState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_connection(socket, state))
}

/// Handle a single WebSocket connection.
async fn handle_connection(socket: WebSocket, state: Arc<GatewayState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast events
    let mut broadcast_rx = state.broadcast.subscribe();

    let session_id = uuid::Uuid::new_v4().to_string();
    let mut authenticated = false;
    let mut user_id: Option<uuid::Uuid> = None;
    let _sequence: u64 = 0;

    // Send hello (prompt client to identify)
    let hello = serde_json::json!({
        "op": "Hello",
        "d": {
            "heartbeat_interval": 45000,
        }
    });
    if sender
        .send(Message::Text(serde_json::to_string(&hello).unwrap().into()))
        .await
        .is_err()
    {
        return;
    }

    // Spawn task to forward broadcast events to this client
    let send_task = tokio::spawn(async move {
        while let Ok(event) = broadcast_rx.recv().await {
            let msg = serde_json::json!({
                "op": "Dispatch",
                "d": {
                    "event": event.event_type,
                    "data": event.data,
                    "sequence": 0, // TODO: per-session sequence
                }
            });

            if sender
                .send(Message::Text(serde_json::to_string(&msg).unwrap().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Receive loop
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                // Parse incoming gateway message
                if let Ok(gateway_msg) = serde_json::from_str::<GatewayMessage>(&text) {
                    match gateway_msg {
                        GatewayMessage::Identify { token } => {
                            // Validate token
                            let config = nexus_common::config::get();
                            match nexus_api::auth::validate_token(&token, &config.auth.jwt_secret) {
                                Ok(claims) => {
                                    authenticated = true;
                                    user_id = claims.sub.parse().ok();
                                    tracing::info!(
                                        session = %session_id,
                                        user = %claims.username,
                                        "Client authenticated on gateway"
                                    );
                                    // TODO: Send READY event with user data
                                }
                                Err(_) => {
                                    tracing::warn!(session = %session_id, "Invalid token on gateway");
                                    // TODO: Send InvalidSession
                                }
                            }
                        }
                        GatewayMessage::Heartbeat { timestamp: _ } => {
                            // Acknowledge heartbeat
                            // TODO: Send HeartbeatAck
                        }
                        GatewayMessage::TypingStart { channel_id } => {
                            if authenticated {
                                // Broadcast typing indicator
                                let _ = state.broadcast.send(GatewayEvent {
                                    event_type: "TYPING_START".into(),
                                    data: serde_json::json!({
                                        "channel_id": channel_id,
                                        "user_id": user_id,
                                        "timestamp": chrono::Utc::now().timestamp(),
                                    }),
                                    server_id: None,
                                    channel_id: channel_id.parse().ok(),
                                    user_id,
                                });
                            }
                        }
                        _ => {
                            // Handle other opcodes
                        }
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup
    send_task.abort();
    tracing::info!(session = %session_id, "Client disconnected from gateway");
}
