//! Role model — granular permission system.
//!
//! Nexus roles are more flexible than Discord's:
//! - Fine-grained channel-level overrides
//! - No arbitrary role limit (configurable by server)
//! - Color, icon, and ordering are all free (no Nitro requirement)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// A role within a server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub server_id: Uuid,

    /// Role name
    pub name: String,

    /// Role color (hex as integer, e.g., 0xFF5733)
    pub color: Option<i32>,

    /// Whether this role is displayed separately in the member list
    pub hoist: bool,

    /// Role icon (emoji or image key)
    pub icon: Option<String>,

    /// Position in the role hierarchy (higher = more power)
    pub position: i32,

    /// Permission bitfield
    pub permissions: i64,

    /// Whether this role can be @mentioned
    pub mentionable: bool,

    /// Whether this is the @everyone role (one per server)
    pub is_default: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateRoleRequest {
    #[validate(length(min = 1, max = 100, message = "Role name must be 1-100 characters"))]
    pub name: String,

    pub color: Option<i32>,
    pub hoist: Option<bool>,
    pub mentionable: Option<bool>,
    pub permissions: Option<i64>,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateRoleRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    pub color: Option<i32>,
    pub hoist: Option<bool>,
    pub mentionable: Option<bool>,
    pub permissions: Option<i64>,
    pub position: Option<i32>,
}
