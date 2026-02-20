//! Member model — a user's membership in a specific server.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a user's membership in a server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub user_id: Uuid,
    pub server_id: Uuid,

    /// Server-specific nickname
    pub nickname: Option<String>,

    /// Server-specific avatar override
    pub avatar: Option<String>,

    /// Role IDs assigned to this member
    pub roles: Vec<Uuid>,

    /// Whether this member is server-muted (by admin)
    pub muted: bool,

    /// Whether this member is server-deafened (by admin)
    pub deafened: bool,

    /// When the user joined this server
    pub joined_at: DateTime<Utc>,

    /// Communication timeout (mute until this time)
    pub communication_disabled_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct MemberResponse {
    pub user_id: Uuid,
    pub server_id: Uuid,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub roles: Vec<Uuid>,
    pub joined_at: DateTime<Utc>,
}

impl From<Member> for MemberResponse {
    fn from(m: Member) -> Self {
        Self {
            user_id: m.user_id,
            server_id: m.server_id,
            nickname: m.nickname,
            avatar: m.avatar,
            roles: m.roles,
            joined_at: m.joined_at,
        }
    }
}
