//! Voice state management — tracks who's in which voice channel.
//!
//! This is the authoritative source of truth for voice connections:
//! - Which users are in which voice channels
//! - Mute/deaf/video/screen share state per user
//! - Server-side mute/deaf (moderation)
//!
//! State is held in-memory for speed and mirrored to Redis for
//! multi-node coordination in production.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Voice state for a single user's voice connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceState {
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub server_id: Option<Uuid>,
    pub session_id: String,
    pub self_mute: bool,
    pub self_deaf: bool,
    pub server_mute: bool,
    pub server_deaf: bool,
    pub self_video: bool,
    pub self_stream: bool,
    pub suppress: bool,
    pub speaking: bool,
    pub connected_at: DateTime<Utc>,
}

/// Request to update voice state.
#[derive(Debug, Clone, Deserialize)]
pub struct VoiceStateUpdate {
    pub self_mute: Option<bool>,
    pub self_deaf: Option<bool>,
    pub self_video: Option<bool>,
    pub self_stream: Option<bool>,
}

/// Request from a moderator to server-mute/deaf a user.
#[derive(Debug, Clone, Deserialize)]
pub struct VoiceModAction {
    pub target_user_id: Uuid,
    pub server_mute: Option<bool>,
    pub server_deaf: Option<bool>,
}

/// Manages voice state across all channels.
///
/// Two indexes for fast lookups:
/// - `by_user`: user_id → VoiceState (quick "where is this user?")
/// - `by_channel`: channel_id → [user_id] (quick "who's in this channel?")
#[derive(Clone)]
pub struct VoiceStateManager {
    by_user: Arc<RwLock<HashMap<Uuid, VoiceState>>>,
    by_channel: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,
}

impl VoiceStateManager {
    pub fn new() -> Self {
        Self {
            by_user: Arc::new(RwLock::new(HashMap::new())),
            by_channel: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// User joins a voice channel. If already in another channel, leaves it first.
    /// Returns (new_state, Option<old_channel_id>).
    pub async fn join(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        server_id: Option<Uuid>,
        session_id: String,
    ) -> (VoiceState, Option<Uuid>) {
        let old_channel = self.leave(user_id).await;

        let state = VoiceState {
            user_id,
            channel_id,
            server_id,
            session_id,
            self_mute: false,
            self_deaf: false,
            server_mute: false,
            server_deaf: false,
            self_video: false,
            self_stream: false,
            suppress: false,
            speaking: false,
            connected_at: Utc::now(),
        };

        // Add to user index
        self.by_user.write().await.insert(user_id, state.clone());

        // Add to channel index
        self.by_channel
            .write()
            .await
            .entry(channel_id)
            .or_default()
            .push(user_id);

        tracing::info!(
            user = %user_id,
            channel = %channel_id,
            server = ?server_id,
            "User joined voice channel"
        );

        (state, old_channel)
    }

    /// User leaves their current voice channel. Returns the channel they left.
    pub async fn leave(&self, user_id: Uuid) -> Option<Uuid> {
        let state = self.by_user.write().await.remove(&user_id);

        if let Some(ref s) = state {
            // Remove from channel index
            let mut channels = self.by_channel.write().await;
            if let Some(members) = channels.get_mut(&s.channel_id) {
                members.retain(|u| *u != user_id);
                if members.is_empty() {
                    channels.remove(&s.channel_id);
                }
            }

            tracing::info!(
                user = %user_id,
                channel = %s.channel_id,
                "User left voice channel"
            );
        }

        state.map(|s| s.channel_id)
    }

    /// Update a user's self-mute/deaf/video/stream state.
    pub async fn update_self_state(
        &self,
        user_id: Uuid,
        update: &VoiceStateUpdate,
    ) -> Option<VoiceState> {
        let mut users = self.by_user.write().await;
        if let Some(state) = users.get_mut(&user_id) {
            if let Some(m) = update.self_mute {
                state.self_mute = m;
            }
            if let Some(d) = update.self_deaf {
                state.self_deaf = d;
                // If undeafening, also unmute? Discord does this — we don't force it.
            }
            if let Some(v) = update.self_video {
                state.self_video = v;
            }
            if let Some(s) = update.self_stream {
                state.self_stream = s;
            }
            Some(state.clone())
        } else {
            None
        }
    }

    /// Moderator action: server-mute or server-deaf a user.
    pub async fn apply_mod_action(&self, action: &VoiceModAction) -> Option<VoiceState> {
        let mut users = self.by_user.write().await;
        if let Some(state) = users.get_mut(&action.target_user_id) {
            if let Some(m) = action.server_mute {
                state.server_mute = m;
            }
            if let Some(d) = action.server_deaf {
                state.server_deaf = d;
            }
            Some(state.clone())
        } else {
            None
        }
    }

    /// Update speaking state (from voice activity detection).
    pub async fn set_speaking(&self, user_id: Uuid, speaking: bool) -> Option<VoiceState> {
        let mut users = self.by_user.write().await;
        if let Some(state) = users.get_mut(&user_id) {
            state.speaking = speaking;
            Some(state.clone())
        } else {
            None
        }
    }

    /// Get a user's current voice state.
    pub async fn get_user_state(&self, user_id: Uuid) -> Option<VoiceState> {
        self.by_user.read().await.get(&user_id).cloned()
    }

    /// Get all users in a voice channel.
    pub async fn get_channel_members(&self, channel_id: Uuid) -> Vec<VoiceState> {
        let channels = self.by_channel.read().await;
        let users = self.by_user.read().await;

        channels
            .get(&channel_id)
            .map(|member_ids| {
                member_ids
                    .iter()
                    .filter_map(|uid| users.get(uid).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the count of users in a voice channel.
    pub async fn get_channel_count(&self, channel_id: Uuid) -> usize {
        self.by_channel
            .read()
            .await
            .get(&channel_id)
            .map(|m| m.len())
            .unwrap_or(0)
    }

    /// Check if a user is in any voice channel.
    pub async fn is_in_voice(&self, user_id: Uuid) -> bool {
        self.by_user.read().await.contains_key(&user_id)
    }

    /// Disconnect all users from a channel (e.g., channel deleted).
    pub async fn disconnect_channel(&self, channel_id: Uuid) -> Vec<VoiceState> {
        let member_ids = self
            .by_channel
            .write()
            .await
            .remove(&channel_id)
            .unwrap_or_default();

        let mut disconnected = Vec::new();
        let mut users = self.by_user.write().await;
        for uid in member_ids {
            if let Some(state) = users.remove(&uid) {
                disconnected.push(state);
            }
        }

        if !disconnected.is_empty() {
            tracing::info!(
                channel = %channel_id,
                count = disconnected.len(),
                "Disconnected all users from voice channel"
            );
        }

        disconnected
    }

    /// Get global voice stats.
    pub async fn stats(&self) -> VoiceGlobalStats {
        let users = self.by_user.read().await;
        let channels = self.by_channel.read().await;

        VoiceGlobalStats {
            active_channels: channels.len(),
            total_connections: users.len(),
            streaming_count: users.values().filter(|s| s.self_stream).count(),
            video_count: users.values().filter(|s| s.self_video).count(),
        }
    }
}

impl Default for VoiceStateManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global voice statistics.
#[derive(Debug, Serialize)]
pub struct VoiceGlobalStats {
    pub active_channels: usize,
    pub total_connections: usize,
    pub streaming_count: usize,
    pub video_count: usize,
}
