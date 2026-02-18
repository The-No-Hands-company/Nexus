//! Server (Guild) model — the community container.
//!
//! "Servers" in Nexus are equivalent to Discord "guilds/servers."
//! Self-hostable instances can federate their servers with others.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// A Nexus server (community).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Server {
    pub id: Uuid,

    /// Server name (2-100 chars)
    pub name: String,

    /// Server description (up to 1000 chars)
    pub description: Option<String>,

    /// Icon image key
    pub icon: Option<String>,

    /// Banner image key
    pub banner: Option<String>,

    /// Owner user ID
    pub owner_id: Uuid,

    /// Server region hint (for voice optimization)
    pub region: Option<String>,

    /// Whether the server is public (discoverable) or private (invite-only)
    pub is_public: bool,

    /// Server-level features enabled
    pub features: serde_json::Value,

    /// Server-level settings (JSON blob for flexibility)
    pub settings: serde_json::Value,

    /// Vanity invite code (e.g., nexus.chat/gaming)
    pub vanity_code: Option<String>,

    /// Member count (denormalized for performance)
    pub member_count: i32,

    /// Max file upload size override (server admins can set this)
    pub max_file_size: Option<i64>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateServerRequest {
    #[validate(length(min = 2, max = 100, message = "Server name must be 2-100 characters"))]
    pub name: String,

    #[validate(length(max = 1000))]
    pub description: Option<String>,

    pub is_public: Option<bool>,

    /// Template to clone from (pre-built channel structures)
    pub template: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateServerRequest {
    #[validate(length(min = 2, max = 100))]
    pub name: Option<String>,

    #[validate(length(max = 1000))]
    pub description: Option<String>,

    pub is_public: Option<bool>,

    pub region: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ServerResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub owner_id: Uuid,
    pub region: Option<String>,
    pub is_public: bool,
    pub vanity_code: Option<String>,
    pub member_count: i32,
    pub created_at: DateTime<Utc>,
}

impl From<Server> for ServerResponse {
    fn from(s: Server) -> Self {
        Self {
            id: s.id,
            name: s.name,
            description: s.description,
            icon: s.icon,
            banner: s.banner,
            owner_id: s.owner_id,
            region: s.region,
            is_public: s.is_public,
            vanity_code: s.vanity_code,
            member_count: s.member_count,
            created_at: s.created_at,
        }
    }
}

/// Server invite
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Invite {
    /// Short invite code (e.g., "abc123")
    pub code: String,
    pub server_id: Uuid,
    pub channel_id: Option<Uuid>,
    pub inviter_id: Uuid,
    /// Max uses (None = unlimited)
    pub max_uses: Option<i32>,
    pub uses: i32,
    /// Expiry time (None = never)
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInviteRequest {
    /// Max number of uses (0 = unlimited)
    pub max_uses: Option<i32>,
    /// Duration in seconds (0 = never expires)
    pub max_age_secs: Option<u64>,
}
