//! Voice room — represents a single voice channel session.
//!
//! This is the higher-level abstraction over the SFU room.
//! It manages participant metadata and integrates with the voice state manager.
//! The actual WebRTC media handling is done by the SFU engine.

use crate::state::VoiceState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// A voice room corresponding to a voice channel.
/// Tracks participant metadata independently from the WebRTC layer.
#[derive(Clone)]
pub struct VoiceRoom {
    pub channel_id: Uuid,
    pub server_id: Option<Uuid>,
    /// Participants in this room (user_id → VoiceParticipant)
    participants: Arc<RwLock<HashMap<Uuid, VoiceParticipant>>>,
    /// When the room was created (first user joined)
    pub created_at: DateTime<Utc>,
}

/// A participant in a voice room with extended metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceParticipant {
    pub user_id: Uuid,
    pub session_id: String,
    pub self_mute: bool,
    pub self_deaf: bool,
    pub server_mute: bool,
    pub server_deaf: bool,
    pub video: bool,
    pub screen_share: bool,
    pub speaking: bool,
    /// Whether the user has noise suppression enabled (client-side nnnoiseless)
    pub noise_suppression: bool,
    /// Volume adjustment by the client (0-200%, default 100%)
    pub volume: u8,
    pub joined_at: DateTime<Utc>,
}

impl VoiceRoom {
    pub fn new(channel_id: Uuid) -> Self {
        Self {
            channel_id,
            server_id: None,
            participants: Arc::new(RwLock::new(HashMap::new())),
            created_at: Utc::now(),
        }
    }

    pub fn with_server(mut self, server_id: Uuid) -> Self {
        self.server_id = Some(server_id);
        self
    }

    /// Add a participant to the room.
    pub async fn join(&self, user_id: Uuid, session_id: String) -> VoiceParticipant {
        let participant = VoiceParticipant {
            user_id,
            session_id,
            self_mute: false,
            self_deaf: false,
            server_mute: false,
            server_deaf: false,
            video: false,
            screen_share: false,
            speaking: false,
            noise_suppression: true, // Enabled by default
            volume: 100,
            joined_at: Utc::now(),
        };

        self.participants
            .write()
            .await
            .insert(user_id, participant.clone());

        tracing::info!(
            channel = %self.channel_id,
            user = %user_id,
            "User joined voice room"
        );

        participant
    }

    /// Sync participant state from VoiceState.
    pub async fn sync_from_voice_state(&self, vs: &VoiceState) {
        if let Some(participant) = self.participants.write().await.get_mut(&vs.user_id) {
            participant.self_mute = vs.self_mute;
            participant.self_deaf = vs.self_deaf;
            participant.server_mute = vs.server_mute;
            participant.server_deaf = vs.server_deaf;
            participant.video = vs.self_video;
            participant.screen_share = vs.self_stream;
            participant.speaking = vs.speaking;
        }
    }

    /// Remove a participant from the room.
    pub async fn leave(&self, user_id: Uuid) -> Option<VoiceParticipant> {
        let removed = self.participants.write().await.remove(&user_id);

        if removed.is_some() {
            tracing::info!(
                channel = %self.channel_id,
                user = %user_id,
                "User left voice room"
            );
        }

        removed
    }

    /// Update a participant's state (mute, deaf, video, etc.).
    pub async fn update_state(
        &self,
        user_id: Uuid,
        self_mute: Option<bool>,
        self_deaf: Option<bool>,
        video: Option<bool>,
        screen_share: Option<bool>,
    ) {
        if let Some(participant) = self.participants.write().await.get_mut(&user_id) {
            if let Some(m) = self_mute {
                participant.self_mute = m;
            }
            if let Some(d) = self_deaf {
                participant.self_deaf = d;
            }
            if let Some(v) = video {
                participant.video = v;
            }
            if let Some(ss) = screen_share {
                participant.screen_share = ss;
            }
        }
    }

    /// Get all participants.
    pub async fn get_participants(&self) -> Vec<VoiceParticipant> {
        self.participants.read().await.values().cloned().collect()
    }

    /// Get a specific participant.
    pub async fn get_participant(&self, user_id: Uuid) -> Option<VoiceParticipant> {
        self.participants.read().await.get(&user_id).cloned()
    }

    /// Get participant count.
    pub async fn participant_count(&self) -> usize {
        self.participants.read().await.len()
    }

    /// Check if room is empty.
    pub async fn is_empty(&self) -> bool {
        self.participants.read().await.is_empty()
    }
}
