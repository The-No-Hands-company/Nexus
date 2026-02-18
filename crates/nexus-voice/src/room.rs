//! Voice room — represents a single voice channel session.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// A voice room corresponding to a voice channel.
#[derive(Clone)]
pub struct VoiceRoom {
    pub channel_id: Uuid,
    /// Participants in this room (user_id → VoiceParticipant)
    participants: Arc<RwLock<HashMap<Uuid, VoiceParticipant>>>,
}

/// A participant in a voice room.
#[derive(Debug, Clone, serde::Serialize)]
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
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

impl VoiceRoom {
    pub fn new(channel_id: Uuid) -> Self {
        Self {
            channel_id,
            participants: Arc::new(RwLock::new(HashMap::new())),
        }
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
            joined_at: chrono::Utc::now(),
        };

        self.participants
            .write()
            .await
            .insert(user_id, participant.clone());

        tracing::info!(
            channel = %self.channel_id,
            user = %user_id,
            "User joined voice channel"
        );

        participant
    }

    /// Remove a participant from the room.
    pub async fn leave(&self, user_id: Uuid) -> Option<VoiceParticipant> {
        let removed = self.participants.write().await.remove(&user_id);

        if removed.is_some() {
            tracing::info!(
                channel = %self.channel_id,
                user = %user_id,
                "User left voice channel"
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

    /// Get participant count.
    pub async fn participant_count(&self) -> usize {
        self.participants.read().await.len()
    }

    /// Check if room is empty.
    pub async fn is_empty(&self) -> bool {
        self.participants.read().await.is_empty()
    }
}
