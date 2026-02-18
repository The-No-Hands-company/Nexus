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
use nexus_common::gateway_event::GatewayEvent;
use nexus_db::repository::{channels, members, read_states, servers};
use serde::{Deserialize, Serialize};
use session::SessionManager;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Gateway state.
#[derive(Clone)]
pub struct GatewayState {
    /// Broadcast channel for dispatching events to all connected clients.
    /// In production, this would use Redis pub/sub for multi-node support.
    pub broadcast: broadcast::Sender<GatewayEvent>,
    pub db: nexus_db::Database,
    pub sessions: Arc<SessionManager>,
}

impl GatewayState {
    pub fn new(db: nexus_db::Database) -> Self {
        let (broadcast, _) = broadcast::channel(10_000);
        Self {
            broadcast,
            db,
            sessions: Arc::new(SessionManager::new()),
        }
    }

    /// Create a GatewayState using an externally-created broadcast sender.
    /// This allows the API server to share the same broadcast channel.
    pub fn with_broadcast(
        db: nexus_db::Database,
        broadcast: broadcast::Sender<GatewayEvent>,
    ) -> Self {
        Self {
            broadcast,
            db,
            sessions: Arc::new(SessionManager::new()),
        }
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

// GatewayEvent is imported at the top of the file — re-export it here
// so consumers (nexus-server) can use `nexus_gateway::GatewayEvent`

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
    let mut subscribed_servers: Vec<uuid::Uuid> = Vec::new();

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

    // Spawn task to forward broadcast events to this client.
    // Events are filtered by the user's subscribed servers.
    let subscribed = subscribed_servers.clone();
    let send_task = tokio::spawn({
        let session_id = session_id.clone();
        async move {
            let _ = session_id; // will be used for per-session filtering
            while let Ok(event) = broadcast_rx.recv().await {
                // Filter: only forward events for servers this user is in,
                // or DM events targeting this user, or events with no server scope
                let should_forward = match event.server_id {
                    Some(sid) => subscribed.contains(&sid),
                    None => {
                        // DM or global event — forward if it targets this user or has no target
                        true
                    }
                };

                if !should_forward {
                    continue;
                }

                let msg = serde_json::json!({
                    "op": "Dispatch",
                    "d": {
                        "event": event.event_type,
                        "data": event.data,
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
                            match nexus_common::auth::validate_token(&token, &config.auth.jwt_secret) {
                                Ok(claims) => {
                                    authenticated = true;
                                    let uid: uuid::Uuid = match claims.sub.parse() {
                                        Ok(id) => id,
                                        Err(_) => {
                                            tracing::warn!(session = %session_id, "Invalid user ID in token");
                                            continue;
                                        }
                                    };
                                    user_id = Some(uid);

                                    // Build READY payload
                                    let ready = build_ready_payload(
                                        &state, uid, &session_id, &claims.username,
                                    ).await;

                                    // Track subscribed servers for event filtering
                                    if let Some(srvs) = ready.get("servers").and_then(|s| s.as_array()) {
                                        subscribed_servers = srvs.iter()
                                            .filter_map(|s| s.get("id").and_then(|id| id.as_str()))
                                            .filter_map(|id| id.parse().ok())
                                            .collect();
                                    }

                                    // Register session
                                    state.sessions.register(
                                        session_id.clone(),
                                        uid,
                                        subscribed_servers.clone(),
                                    ).await;

                                    let ready_msg = GatewayMessage::Ready {
                                        session_id: session_id.clone(),
                                        user: ready["user"].clone(),
                                        servers: ready["servers"]
                                            .as_array()
                                            .cloned()
                                            .unwrap_or_default(),
                                    };

                                    // We need to send via broadcast to reach the send_task
                                    // Instead, we'll use the direct pattern below
                                    let _ = state.broadcast.send(GatewayEvent {
                                        event_type: "__READY__".into(),
                                        data: serde_json::to_value(&ready_msg).unwrap_or_default(),
                                        server_id: None,
                                        channel_id: None,
                                        user_id: Some(uid),
                                    });

                                    tracing::info!(
                                        session = %session_id,
                                        user = %claims.username,
                                        servers = subscribed_servers.len(),
                                        "Client authenticated — READY sent"
                                    );
                                }
                                Err(_) => {
                                    tracing::warn!(session = %session_id, "Invalid token on gateway");
                                    let invalid = serde_json::json!({
                                        "op": "InvalidSession",
                                        "d": null
                                    });
                                    let _ = state.broadcast.send(GatewayEvent {
                                        event_type: "__INVALID_SESSION__".into(),
                                        data: invalid,
                                        server_id: None,
                                        channel_id: None,
                                        user_id: None,
                                    });
                                }
                            }
                        }
                        GatewayMessage::Heartbeat { timestamp: _ } => {
                            // Send HeartbeatAck
                            let ack = GatewayEvent {
                                event_type: "__HEARTBEAT_ACK__".into(),
                                data: serde_json::json!({
                                    "op": "HeartbeatAck",
                                    "d": { "timestamp": chrono::Utc::now().timestamp_millis() }
                                }),
                                server_id: None,
                                channel_id: None,
                                user_id,
                            };
                            let _ = state.broadcast.send(ack);
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
                        GatewayMessage::PresenceUpdate { status, custom_status } => {
                            if let Some(uid) = user_id {
                                // Update presence in DB
                                let _ = nexus_db::repository::users::update_presence(
                                    &state.db.pg, uid, &status,
                                ).await;

                                // Broadcast presence update to all servers this user is in
                                let _ = state.broadcast.send(GatewayEvent {
                                    event_type: "PRESENCE_UPDATE".into(),
                                    data: serde_json::json!({
                                        "user_id": uid,
                                        "status": status,
                                        "custom_status": custom_status,
                                    }),
                                    server_id: None,
                                    channel_id: None,
                                    user_id: Some(uid),
                                });
                            }
                        }
                        GatewayMessage::VoiceStateUpdate {
                            server_id: vs_server_id,
                            channel_id: vs_channel_id,
                            self_mute,
                            self_deaf,
                        } => {
                            if let Some(uid) = user_id {
                                // Relay voice state update through the broadcast channel.
                                // The actual voice connection is managed by nexus-voice;
                                // the gateway just broadcasts state changes to other clients.
                                let server_uuid = vs_server_id
                                    .as_ref()
                                    .and_then(|s| s.parse::<uuid::Uuid>().ok());
                                let channel_uuid = vs_channel_id
                                    .as_ref()
                                    .and_then(|c| c.parse::<uuid::Uuid>().ok());

                                let _ = state.broadcast.send(GatewayEvent {
                                    event_type: "VOICE_STATE_UPDATE".into(),
                                    data: serde_json::json!({
                                        "user_id": uid,
                                        "server_id": vs_server_id,
                                        "channel_id": vs_channel_id,
                                        "self_mute": self_mute,
                                        "self_deaf": self_deaf,
                                    }),
                                    server_id: server_uuid,
                                    channel_id: channel_uuid,
                                    user_id: Some(uid),
                                });

                                tracing::debug!(
                                    user = %uid,
                                    channel = ?vs_channel_id,
                                    "Voice state update relayed"
                                );
                            }
                        }
                        _ => {
                            // Unknown opcode — ignore
                        }
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup — remove session
    state.sessions.remove(&session_id).await;
    if let Some(uid) = user_id {
        // If this was the user's last session, set presence to offline
        if !state.sessions.is_online(uid).await {
            let _ = nexus_db::repository::users::update_presence(
                &state.db.pg, uid, "offline",
            ).await;
            let _ = state.broadcast.send(GatewayEvent {
                event_type: "PRESENCE_UPDATE".into(),
                data: serde_json::json!({
                    "user_id": uid,
                    "status": "offline",
                }),
                server_id: None,
                channel_id: None,
                user_id: Some(uid),
            });
        }
    }

    send_task.abort();
    tracing::info!(session = %session_id, "Client disconnected from gateway");
}

/// Build the READY payload for a newly authenticated user.
/// Contains: user profile, server list with channels, read states.
async fn build_ready_payload(
    state: &GatewayState,
    uid: uuid::Uuid,
    session_id: &str,
    _username: &str,
) -> serde_json::Value {
    // Fetch user profile
    let user = nexus_db::repository::users::find_by_id(&state.db.pg, uid)
        .await
        .ok()
        .flatten();

    // Fetch user's servers
    let user_servers = servers::list_user_servers(&state.db.pg, uid)
        .await
        .unwrap_or_default();

    // For each server, fetch channels
    let mut server_payloads = Vec::new();
    for server in &user_servers {
        let server_channels = channels::list_server_channels(&state.db.pg, server.id)
            .await
            .unwrap_or_default();

        let member = members::find_member(&state.db.pg, uid, server.id)
            .await
            .ok()
            .flatten();

        server_payloads.push(serde_json::json!({
            "id": server.id,
            "name": server.name,
            "icon": server.icon,
            "owner_id": server.owner_id,
            "member_count": server.member_count,
            "channels": server_channels.iter().map(|c| serde_json::json!({
                "id": c.id,
                "name": c.name,
                "channel_type": c.channel_type,
                "position": c.position,
                "parent_id": c.parent_id,
                "last_message_id": c.last_message_id,
                "topic": c.topic,
                "nsfw": c.nsfw,
            })).collect::<Vec<_>>(),
            "member": member.map(|m| serde_json::json!({
                "nickname": m.nickname,
                "roles": m.roles,
                "joined_at": m.joined_at,
            })),
        }));
    }

    // Fetch DM channels
    let dm_channels = sqlx::query_as::<_, nexus_common::models::channel::Channel>(
        r#"
        SELECT c.* FROM channels c
        INNER JOIN dm_participants dp ON dp.channel_id = c.id
        WHERE dp.user_id = $1 AND c.channel_type IN ('dm', 'group_dm')
        ORDER BY c.updated_at DESC
        "#,
    )
    .bind(uid)
    .fetch_all(&state.db.pg)
    .await
    .unwrap_or_default();

    // Fetch read states
    let user_read_states = read_states::get_all_read_states(&state.db.pg, uid)
        .await
        .unwrap_or_default();

    serde_json::json!({
        "session_id": session_id,
        "user": user.map(|u| serde_json::json!({
            "id": u.id,
            "username": u.username,
            "display_name": u.display_name,
            "avatar": u.avatar,
            "bio": u.bio,
            "status": u.status,
            "presence": u.presence,
            "flags": u.flags,
        })),
        "servers": server_payloads,
        "dm_channels": dm_channels.iter().map(|c| serde_json::json!({
            "id": c.id,
            "channel_type": c.channel_type,
            "last_message_id": c.last_message_id,
        })).collect::<Vec<_>>(),
        "read_states": user_read_states.iter().map(|rs| serde_json::json!({
            "channel_id": rs.channel_id,
            "last_read_message_id": rs.last_read_message_id,
            "mention_count": rs.mention_count,
        })).collect::<Vec<_>>(),
    })
}
