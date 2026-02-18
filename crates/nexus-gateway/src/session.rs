//! Gateway session management.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Tracks all active gateway sessions.
pub struct SessionManager {
    /// Map of session_id → Session
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    /// Map of user_id → Vec<session_id> (a user can have multiple sessions/devices)
    user_sessions: Arc<RwLock<HashMap<Uuid, Vec<String>>>>,
}

pub struct Session {
    pub session_id: String,
    pub user_id: Uuid,
    pub sequence: u64,
    /// Server IDs this session is subscribed to
    pub subscribed_servers: Vec<Uuid>,
    /// Last heartbeat time
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            user_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new session.
    pub async fn register(&self, session_id: String, user_id: Uuid, servers: Vec<Uuid>) {
        let session = Session {
            session_id: session_id.clone(),
            user_id,
            sequence: 0,
            subscribed_servers: servers,
            last_heartbeat: chrono::Utc::now(),
        };

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);

        self.user_sessions
            .write()
            .await
            .entry(user_id)
            .or_default()
            .push(session_id);
    }

    /// Remove a session.
    pub async fn remove(&self, session_id: &str) {
        if let Some(session) = self.sessions.write().await.remove(session_id) {
            if let Some(sessions) = self.user_sessions.write().await.get_mut(&session.user_id) {
                sessions.retain(|s| s != session_id);
            }
        }
    }

    /// Get all session IDs for a user.
    pub async fn get_user_sessions(&self, user_id: Uuid) -> Vec<String> {
        self.user_sessions
            .read()
            .await
            .get(&user_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if a user is online (has at least one active session).
    pub async fn is_online(&self, user_id: Uuid) -> bool {
        self.user_sessions
            .read()
            .await
            .get(&user_id)
            .is_some_and(|sessions| !sessions.is_empty())
    }

    /// Get total active sessions count.
    pub async fn active_count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
