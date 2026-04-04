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

    /// Whether TOTP two-factor authentication is enabled.
    #[serde(skip_serializing)]
    pub totp_enabled: bool,

    /// For federated (remote) users: the home server name (e.g. `nexus.other.tld`).
    /// `None` for local users.
    pub server_name: Option<String>,

    /// `true` when this is a shadow account representing a user on another server.
    #[serde(default)]
    pub is_remote: bool,

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
    /// Instance administrator — can manage federation, instance settings, etc.
    pub const INSTANCE_ADMIN: i64 = 1 << 7;
    /// Marketplace reviewer — can review and approve Store plugins
    pub const MARKETPLACE_REVIEWER: i64 = 1 << 8;
    /// Marketplace moderator — can quarantine, takedown, and handle reports
    pub const MARKETPLACE_MODERATOR: i64 = 1 << 9;
    /// Creator account — allows publishing to marketplace
    pub const MARKETPLACE_CREATOR: i64 = 1 << 10;
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

    /// URL of the user's avatar image.
    #[validate(url)]
    #[validate(length(max = 512))]
    #[serde(default)]
    pub avatar: Option<String>,
}

use std::sync::LazyLock;
static USERNAME_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::validate_request;

    // ── user_flags bit constants ──────────────────────────────────────────────

    #[test]
    fn all_flag_bits_are_unique() {
        let flags = [
            user_flags::STAFF,
            user_flags::BOT,
            user_flags::EMAIL_VERIFIED,
            user_flags::EARLY_SUPPORTER,
            user_flags::PREMIUM_SUPPORTER,
            user_flags::DISABLED,
            user_flags::SUSPENDED,
            user_flags::INSTANCE_ADMIN,
            user_flags::MARKETPLACE_REVIEWER,
            user_flags::MARKETPLACE_MODERATOR,
            user_flags::MARKETPLACE_CREATOR,
        ];
        let unique: std::collections::HashSet<_> = flags.iter().collect();
        assert_eq!(flags.len(), unique.len(), "duplicate flag bit detected");
    }

    #[test]
    fn flags_are_powers_of_two() {
        let flags = [
            user_flags::STAFF,
            user_flags::BOT,
            user_flags::EMAIL_VERIFIED,
            user_flags::DISABLED,
            user_flags::SUSPENDED,
            user_flags::INSTANCE_ADMIN,
        ];
        for flag in flags {
            assert!(flag > 0 && (flag & (flag - 1)) == 0,
                "flag {flag:#x} is not a power of two");
        }
    }

    #[test]
    fn suspended_and_disabled_are_combinable_with_or() {
        let flags = user_flags::SUSPENDED | user_flags::DISABLED;
        assert_ne!(flags & user_flags::SUSPENDED, 0, "SUSPENDED bit must be set");
        assert_ne!(flags & user_flags::DISABLED, 0, "DISABLED bit must be set");
        assert_eq!(flags & user_flags::STAFF, 0, "STAFF bit must NOT be set");
    }

    #[test]
    fn gateway_ban_check_logic_with_flags() {
        // Mirrors the gateway Identify handler ban check:
        //   (flags & SUSPENDED) == 0 && (flags & DISABLED) == 0
        let clean: i64 = 0;
        let suspended: i64 = user_flags::SUSPENDED;
        let disabled: i64 = user_flags::DISABLED;
        let both: i64 = user_flags::SUSPENDED | user_flags::DISABLED;

        let is_allowed = |f: i64| {
            (f & user_flags::SUSPENDED) == 0 && (f & user_flags::DISABLED) == 0
        };

        assert!(is_allowed(clean),     "clean account must be allowed");
        assert!(!is_allowed(suspended), "suspended account must be blocked");
        assert!(!is_allowed(disabled),  "disabled account must be blocked");
        assert!(!is_allowed(both),      "suspended+disabled must be blocked");
    }

    // ── CreateUserRequest validation ──────────────────────────────────────────

    fn valid_request() -> CreateUserRequest {
        CreateUserRequest {
            username: "alice42".to_string(),
            password: "securepassword".to_string(),
            email: None,
            invite_code: None,
        }
    }

    #[test]
    fn valid_create_user_request_passes_validation() {
        assert!(validate_request(&valid_request()).is_ok());
    }

    #[test]
    fn username_too_short_fails() {
        let req = CreateUserRequest { username: "ab".to_string(), ..valid_request() };
        assert!(validate_request(&req).is_err(), "username < 3 chars must fail");
    }

    #[test]
    fn username_too_long_fails() {
        let req = CreateUserRequest {
            username: "a".repeat(33),
            ..valid_request()
        };
        assert!(validate_request(&req).is_err(), "username > 32 chars must fail");
    }

    #[test]
    fn username_with_invalid_chars_fails() {
        for bad in &["alice!", "alice@", "alice space", "alice.dot", "alice#"] {
            let req = CreateUserRequest { username: bad.to_string(), ..valid_request() };
            assert!(validate_request(&req).is_err(), "{bad} must fail username regex");
        }
    }

    #[test]
    fn username_with_valid_chars_passes() {
        for good in &["alice", "Alice42", "alice_42", "alice-42", "SCREAMING"] {
            let req = CreateUserRequest { username: good.to_string(), ..valid_request() };
            assert!(validate_request(&req).is_ok(), "{good} must pass username regex");
        }
    }

    #[test]
    fn password_too_short_fails() {
        let req = CreateUserRequest { password: "short".to_string(), ..valid_request() };
        assert!(validate_request(&req).is_err(), "password < 8 chars must fail");
    }

    #[test]
    fn password_too_long_fails() {
        let req = CreateUserRequest { password: "x".repeat(129), ..valid_request() };
        assert!(validate_request(&req).is_err(), "password > 128 chars must fail");
    }

    #[test]
    fn valid_email_passes() {
        let req = CreateUserRequest {
            email: Some("alice@example.com".to_string()),
            ..valid_request()
        };
        assert!(validate_request(&req).is_ok());
    }

    #[test]
    fn invalid_email_fails() {
        let req = CreateUserRequest {
            email: Some("not-an-email".to_string()),
            ..valid_request()
        };
        assert!(validate_request(&req).is_err(), "invalid email must fail");
    }
}
