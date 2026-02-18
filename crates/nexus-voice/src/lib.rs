//! # nexus-voice
//!
//! Voice/Video server for Nexus using WebRTC.
//!
//! Architecture: SFU (Selective Forwarding Unit)
//! - Each voice channel is a "room"
//! - Server receives media from each participant and forwards to others
//! - No mixing on server (saves CPU, allows client-side volume control)
//! - Opus codec for audio, VP9 for video/screen share
//!
//! Features that Discord users have been begging for:
//! - 1080p60 screen share for EVERYONE (no Nitro gate)
//! - Per-user volume control (client-side, since SFU)
//! - Noise suppression (RNNoise/nnnoiseless)
//! - Recording with visible consent indicator
//! - Spatial audio (positional, great for gaming)
//! - Low latency (<50ms target)
//!
//! ## Module Layout
//!
//! - [`sfu`] — WebRTC SFU engine (str0m-based, handles media forwarding)
//! - [`state`] — Voice state manager (who's in which channel, mute/deaf)
//! - [`handler`] — WebSocket signaling handler (SDP/ICE exchange)
//! - [`room`] — Voice room abstraction (participant tracking)
//! - [`signaling`] — Signaling message types

pub mod handler;
pub mod room;
pub mod sfu;
pub mod signaling;
pub mod state;

use handler::VoiceServerState;
use nexus_common::gateway_event::GatewayEvent;
use sfu::SfuManager;
use state::VoiceStateManager;
use std::net::IpAddr;
use tokio::sync::broadcast;

/// Voice server — the top-level coordinator for all voice functionality.
///
/// Integrates:
/// - SFU engine (WebRTC media relay)
/// - Voice state manager (presence tracking)
/// - Signaling WebSocket handler
#[derive(Clone)]
pub struct VoiceServer {
    pub state: VoiceServerState,
}

impl VoiceServer {
    /// Create a new voice server.
    ///
    /// # Arguments
    /// - `db` — Database connection for checking permissions
    /// - `gateway_tx` — Broadcast sender to push voice events to the main gateway
    /// - `local_ip` — Local IP address for binding UDP sockets (SFU)
    pub fn new(
        db: nexus_db::Database,
        gateway_tx: broadcast::Sender<GatewayEvent>,
        local_ip: IpAddr,
    ) -> Self {
        let sfu = SfuManager::new(local_ip);
        let voice_state = VoiceStateManager::new();

        let state = VoiceServerState {
            sfu,
            voice_state,
            gateway_tx,
            db,
        };

        Self { state }
    }

    /// Build the Axum router for the voice signaling WebSocket.
    pub fn build_router(&self) -> axum::Router {
        handler::build_router(self.state.clone())
    }

    /// Get voice statistics.
    pub async fn stats(&self) -> VoiceStats {
        let state_stats = self.state.voice_state.stats().await;
        let sfu_rooms = self.state.sfu.active_room_count().await;

        VoiceStats {
            active_channels: state_stats.active_channels,
            total_connections: state_stats.total_connections,
            sfu_rooms,
            streaming_count: state_stats.streaming_count,
            video_count: state_stats.video_count,
        }
    }
}


#[derive(Debug, serde::Serialize)]
pub struct VoiceStats {
    pub active_channels: usize,
    pub total_connections: usize,
    pub sfu_rooms: usize,
    pub streaming_count: usize,
    pub video_count: usize,
}
