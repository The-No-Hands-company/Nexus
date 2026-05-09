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
        if let Some(session) = self.sessions.write().await.remove(session_id)
            && let Some(sessions) = self.user_sessions.write().await.get_mut(&session.user_id)
        {
            sessions.retain(|s| s != session_id);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn uid() -> Uuid {
        Uuid::new_v4()
    }
    fn sid() -> String {
        Uuid::new_v4().to_string()
    }

    // ── register / active_count ───────────────────────────────────────────────

    #[tokio::test]
    async fn register_increments_active_count() {
        let mgr = SessionManager::new();
        assert_eq!(mgr.active_count().await, 0);
        mgr.register(sid(), uid(), vec![]).await;
        assert_eq!(mgr.active_count().await, 1);
        mgr.register(sid(), uid(), vec![]).await;
        assert_eq!(mgr.active_count().await, 2);
    }

    #[tokio::test]
    async fn register_same_user_two_devices_both_counted() {
        let mgr = SessionManager::new();
        let user = uid();
        mgr.register(sid(), user, vec![]).await;
        mgr.register(sid(), user, vec![]).await;

        assert_eq!(mgr.active_count().await, 2);
        assert_eq!(mgr.get_user_sessions(user).await.len(), 2);
    }

    // ── is_online ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn is_online_false_before_register() {
        let mgr = SessionManager::new();
        assert!(!mgr.is_online(uid()).await);
    }

    #[tokio::test]
    async fn is_online_true_after_register() {
        let mgr = SessionManager::new();
        let user = uid();
        mgr.register(sid(), user, vec![]).await;
        assert!(mgr.is_online(user).await);
    }

    #[tokio::test]
    async fn is_online_false_after_all_sessions_removed() {
        let mgr = SessionManager::new();
        let user = uid();
        let s1 = sid();
        let s2 = sid();

        mgr.register(s1.clone(), user, vec![]).await;
        mgr.register(s2.clone(), user, vec![]).await;

        mgr.remove(&s1).await;
        assert!(mgr.is_online(user).await, "still online with one session");

        mgr.remove(&s2).await;
        assert!(
            !mgr.is_online(user).await,
            "offline after all sessions removed"
        );
    }

    // ── remove ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn remove_decrements_active_count() {
        let mgr = SessionManager::new();
        let session = sid();
        mgr.register(session.clone(), uid(), vec![]).await;

        mgr.remove(&session).await;
        assert_eq!(mgr.active_count().await, 0);
    }

    #[tokio::test]
    async fn remove_nonexistent_session_is_no_op() {
        let mgr = SessionManager::new();
        mgr.remove("no-such-session").await; // must not panic
        assert_eq!(mgr.active_count().await, 0);
    }

    #[tokio::test]
    async fn remove_only_removes_specified_session() {
        let mgr = SessionManager::new();
        let user = uid();
        let s1 = sid();
        let s2 = sid();

        mgr.register(s1.clone(), user, vec![]).await;
        mgr.register(s2.clone(), user, vec![]).await;
        mgr.remove(&s1).await;

        assert_eq!(mgr.active_count().await, 1);
        let remaining = mgr.get_user_sessions(user).await;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0], s2);
    }

    // ── get_user_sessions ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_user_sessions_empty_for_unknown_user() {
        let mgr = SessionManager::new();
        assert!(mgr.get_user_sessions(uid()).await.is_empty());
    }

    #[tokio::test]
    async fn get_user_sessions_returns_all_session_ids() {
        let mgr = SessionManager::new();
        let user = uid();
        let s1 = sid();
        let s2 = sid();
        let s3 = sid();

        mgr.register(s1.clone(), user, vec![]).await;
        mgr.register(s2.clone(), user, vec![]).await;
        mgr.register(s3.clone(), user, vec![]).await;

        let sessions = mgr.get_user_sessions(user).await;
        assert_eq!(sessions.len(), 3);
        assert!(sessions.contains(&s1));
        assert!(sessions.contains(&s2));
        assert!(sessions.contains(&s3));
    }

    // ── different users are isolated ──────────────────────────────────────────

    #[tokio::test]
    async fn sessions_are_isolated_between_users() {
        let mgr = SessionManager::new();
        let u1 = uid();
        let u2 = uid();

        mgr.register(sid(), u1, vec![]).await;
        mgr.register(sid(), u2, vec![]).await;

        assert_eq!(mgr.get_user_sessions(u1).await.len(), 1);
        assert_eq!(mgr.get_user_sessions(u2).await.len(), 1);

        // u1 going offline should not affect u2
        for s in mgr.get_user_sessions(u1).await {
            mgr.remove(&s).await;
        }
        assert!(!mgr.is_online(u1).await);
        assert!(mgr.is_online(u2).await);
    }
}
