//! Voice signaling — WebSocket-based negotiation for WebRTC connections.
//!
//! The signaling server handles:
//! - SDP offer/answer exchange
//! - ICE candidate exchange
//! - Room membership management
//!
//! The actual media (audio/video) flows directly between clients and the SFU
//! via WebRTC, NOT through this signaling channel.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Signaling messages between voice client and server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SignalingMessage {
    /// Client → Server: Join a voice channel
    Join {
        channel_id: Uuid,
        server_id: Uuid,
    },

    /// Server → Client: You've joined, here are the current participants
    Joined {
        channel_id: Uuid,
        participants: Vec<serde_json::Value>,
        /// TURN/STUN server configuration
        ice_servers: Vec<IceServer>,
    },

    /// Client → Server: SDP offer (to establish WebRTC connection)
    Offer {
        sdp: String,
    },

    /// Server → Client: SDP answer
    Answer {
        sdp: String,
    },

    /// Bidirectional: ICE candidate
    IceCandidate {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_m_line_index: Option<u32>,
    },

    /// Client → Server: Leave voice channel
    Leave,

    /// Server → Client: Another user joined/left
    ParticipantJoined {
        user_id: Uuid,
    },

    ParticipantLeft {
        user_id: Uuid,
    },

    /// Server → Client: Speaking state changed
    Speaking {
        user_id: Uuid,
        speaking: bool,
    },
}

/// ICE server configuration (STUN/TURN).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

impl IceServer {
    /// Default STUN servers (free, public).
    pub fn default_stun() -> Vec<Self> {
        vec![
            Self {
                urls: vec!["stun:stun.l.google.com:19302".into()],
                username: None,
                credential: None,
            },
            Self {
                urls: vec!["stun:stun1.l.google.com:19302".into()],
                username: None,
                credential: None,
            },
        ]
    }
}
