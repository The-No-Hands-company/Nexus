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
//! - Noise suppression (RNNoise)
//! - Recording with visible consent indicator
//! - Spatial audio (positional, great for gaming)
//! - Low latency (<50ms target)

pub mod room;
pub mod signaling;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Voice server state.
#[derive(Clone)]
pub struct VoiceServer {
    /// Active voice rooms (channel_id → Room)
    rooms: Arc<RwLock<HashMap<Uuid, room::VoiceRoom>>>,
}

impl VoiceServer {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a voice room for a channel.
    pub async fn get_or_create_room(&self, channel_id: Uuid) -> room::VoiceRoom {
        let mut rooms = self.rooms.write().await;
        rooms
            .entry(channel_id)
            .or_insert_with(|| room::VoiceRoom::new(channel_id))
            .clone()
    }

    /// Remove an empty room.
    pub async fn cleanup_room(&self, channel_id: Uuid) {
        let mut rooms = self.rooms.write().await;
        if let Some(room) = rooms.get(&channel_id) {
            if room.is_empty().await {
                rooms.remove(&channel_id);
            }
        }
    }

    /// Get stats about active voice rooms.
    pub async fn stats(&self) -> VoiceStats {
        let rooms = self.rooms.read().await;
        let total_rooms = rooms.len();
        let mut total_participants = 0;

        for room in rooms.values() {
            total_participants += room.participant_count().await;
        }

        VoiceStats {
            active_rooms: total_rooms,
            total_participants,
        }
    }
}

impl Default for VoiceServer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct VoiceStats {
    pub active_rooms: usize,
    pub total_participants: usize,
}
