//! User model — the identity layer.
//!
//! Users in Nexus are pseudonymous by default. No real name required,
//! no phone number, no government ID. Just a username and optional email.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// A Nexus user account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user ID (UUID v7 — time-sortable)
    pub id: Uuid,

    /// Unique username (3-32 chars, alphanumeric + underscores)
    pub username: String,

    /// Display name (optional, up to 64 chars)
    pub display_name: Option<String>,

    /// Email (optional — only needed for password reset, not for registration)
    #[serde(skip_serializing)]
    pub email: Option<String>,

    /// Argon2id password hash
    #[serde(skip_serializing)]
    pub password_hash: String,

    /// Avatar file key (S3/MinIO path)
    pub avatar: Option<String>,

    /// Banner image key
    pub banner: Option<String>,

    /// Short bio / about me (up to 190 chars)
    pub bio: Option<String>,

    /// User-set status message
    pub status: Option<String>,

    /// Online presence state
    pub presence: UserPresence,

    /// User flags (bitfield: staff, verified, bot, etc.)
    pub flags: i64,

    /// Account creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last profile update
    pub updated_at: DateTime<Utc>,
}

/// Presence states — what users want that Discord almost got right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserPresence {
    /// User is actively connected
    Online,
    /// User is connected but idle (configurable threshold)
    Idle,
    /// User wants no interruptions — respects notification rules
    DoNotDisturb,
    /// User appears offline to others (but can still see everything)
    Invisible,
    /// User is not connected
    Offline,
}

/// Bitflags for user account flags.
pub mod user_flags {
    /// Nexus team member
    pub const STAFF: i64 = 1 << 0;
    /// Bot account
    pub const BOT: i64 = 1 << 1;
    /// Email verified
    pub const EMAIL_VERIFIED: i64 = 1 << 2;
    /// Early supporter
    pub const EARLY_SUPPORTER: i64 = 1 << 3;
    /// Server booster (if we implement optional boosting)
    pub const PREMIUM_SUPPORTER: i64 = 1 << 4;
    /// Account disabled by user
    pub const DISABLED: i64 = 1 << 5;
    /// Account suspended by moderation
    pub const SUSPENDED: i64 = 1 << 6;
}

/// Registration request — minimal by design. No ID, no phone, no nonsense.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 32, message = "Username must be 3-32 characters"))]
    #[validate(regex(
        path = *USERNAME_REGEX,
        message = "Username can only contain letters, numbers, underscores, and hyphens"
    ))]
    pub username: String,

    #[validate(length(min = 8, max = 128, message = "Password must be 8-128 characters"))]
    pub password: String,

    /// Optional email — for password recovery only, not required to use the platform
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,

    /// Optional invite code
    pub invite_code: Option<String>,
}

/// Login request
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 3, max = 32))]
    pub username: String,

    #[validate(length(min = 8, max = 128))]
    pub password: String,
}

/// Safe user representation for API responses (no sensitive fields)
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub banner: Option<String>,
    pub bio: Option<String>,
    pub status: Option<String>,
    pub presence: UserPresence,
    pub flags: i64,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            display_name: u.display_name,
            avatar: u.avatar,
            banner: u.banner,
            bio: u.bio,
            status: u.status,
            presence: u.presence,
            flags: u.flags,
            created_at: u.created_at,
        }
    }
}

/// Update profile request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(length(min = 3, max = 32))]
    pub username: Option<String>,

    #[validate(length(max = 64))]
    pub display_name: Option<String>,

    #[validate(length(max = 190))]
    pub bio: Option<String>,

    #[validate(length(max = 128))]
    pub status: Option<String>,

    pub presence: Option<UserPresence>,
}

use std::sync::LazyLock;
static USERNAME_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());
