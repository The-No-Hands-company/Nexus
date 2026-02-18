//! Permission system — granular, transparent, no hidden gotchas.
//!
//! Permissions in Nexus use a bitfield system (like Discord) but with clearer semantics
//! and more granular controls that users have been asking for.

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    /// Server-level and channel-level permissions.
    ///
    /// Each permission is a single bit. Roles combine permissions via OR.
    /// Channel overrides can explicitly ALLOW or DENY specific permissions.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Permissions: i64 {
        // === General ===
        /// View channels and read messages
        const VIEW_CHANNEL          = 1 << 0;
        /// Manage server settings, channels, roles
        const MANAGE_SERVER         = 1 << 1;
        /// Manage specific channels (edit, delete)
        const MANAGE_CHANNELS       = 1 << 2;
        /// Manage roles below your highest role
        const MANAGE_ROLES          = 1 << 3;
        /// Create invite links
        const CREATE_INVITES        = 1 << 4;
        /// Kick members
        const KICK_MEMBERS          = 1 << 5;
        /// Ban members
        const BAN_MEMBERS           = 1 << 6;
        /// View audit log
        const VIEW_AUDIT_LOG        = 1 << 7;
        /// Change own nickname
        const CHANGE_NICKNAME       = 1 << 8;
        /// Change other members' nicknames
        const MANAGE_NICKNAMES      = 1 << 9;
        /// Manage custom emojis and stickers
        const MANAGE_EMOJIS         = 1 << 10;
        /// Manage webhooks
        const MANAGE_WEBHOOKS       = 1 << 11;

        // === Text ===
        /// Send messages in text channels
        const SEND_MESSAGES         = 1 << 12;
        /// Send messages in threads
        const SEND_MESSAGES_IN_THREADS = 1 << 13;
        /// Create public threads
        const CREATE_PUBLIC_THREADS = 1 << 14;
        /// Create private threads
        const CREATE_PRIVATE_THREADS = 1 << 15;
        /// Manage threads (archive, delete, edit)
        const MANAGE_THREADS        = 1 << 16;
        /// Embed links (auto-preview)
        const EMBED_LINKS           = 1 << 17;
        /// Attach files
        const ATTACH_FILES          = 1 << 18;
        /// Add reactions to messages
        const ADD_REACTIONS         = 1 << 19;
        /// Use external emojis from other servers
        const USE_EXTERNAL_EMOJIS   = 1 << 20;
        /// Mention @everyone and @here
        const MENTION_EVERYONE      = 1 << 21;
        /// Manage messages (delete others' messages, pin)
        const MANAGE_MESSAGES       = 1 << 22;
        /// Read message history
        const READ_MESSAGE_HISTORY  = 1 << 23;
        /// Use slash commands and bot interactions
        const USE_COMMANDS          = 1 << 24;

        // === Voice ===
        /// Connect to voice channels
        const CONNECT               = 1 << 25;
        /// Speak in voice channels
        const SPEAK                 = 1 << 26;
        /// Use video in voice channels
        const VIDEO                 = 1 << 27;
        /// Mute other members
        const MUTE_MEMBERS          = 1 << 28;
        /// Deafen other members
        const DEAFEN_MEMBERS        = 1 << 29;
        /// Move members between voice channels
        const MOVE_MEMBERS          = 1 << 30;
        /// Use voice activity detection (vs push-to-talk only)
        const USE_VAD               = 1 << 31;
        /// Use screen share
        const SCREEN_SHARE          = 1 << 32;
        /// Use stage speaker
        const STAGE_SPEAKER         = 1 << 33;

        // === Nexus-specific (stuff Discord never gave us) ===
        /// Record voice channel (with visible consent indicator)
        const RECORD_VOICE          = 1 << 34;
        /// Create and manage polls
        const MANAGE_POLLS          = 1 << 35;
        /// Create and manage events/scheduled activities
        const MANAGE_EVENTS         = 1 << 36;
        /// Pin messages
        const PIN_MESSAGES          = 1 << 37;
        /// Manage server plugins/extensions
        const MANAGE_PLUGINS        = 1 << 38;
        /// View server analytics/insights
        const VIEW_ANALYTICS        = 1 << 39;

        // === Meta ===
        /// Server owner / administrator (all permissions)
        const ADMINISTRATOR         = 1 << 40;
    }
}

impl Permissions {
    /// Default permissions for @everyone role in a new server.
    pub fn default_everyone() -> Self {
        Self::VIEW_CHANNEL
            | Self::SEND_MESSAGES
            | Self::SEND_MESSAGES_IN_THREADS
            | Self::CREATE_PUBLIC_THREADS
            | Self::EMBED_LINKS
            | Self::ATTACH_FILES
            | Self::ADD_REACTIONS
            | Self::USE_EXTERNAL_EMOJIS
            | Self::READ_MESSAGE_HISTORY
            | Self::USE_COMMANDS
            | Self::CONNECT
            | Self::SPEAK
            | Self::VIDEO
            | Self::USE_VAD
            | Self::SCREEN_SHARE
            | Self::CHANGE_NICKNAME
            | Self::CREATE_INVITES
    }

    /// Check if administrator (overrides all other checks).
    pub fn is_admin(&self) -> bool {
        self.contains(Self::ADMINISTRATOR)
    }

    /// Check if a user with these permissions can perform an action.
    pub fn has(&self, required: Permissions) -> bool {
        self.is_admin() || self.contains(required)
    }
}

/// Channel-level permission override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOverwrite {
    /// The role or user ID this override applies to
    pub target_id: uuid::Uuid,
    /// Whether this targets a role or user
    pub target_type: OverwriteType,
    /// Permissions explicitly allowed
    pub allow: i64,
    /// Permissions explicitly denied
    pub deny: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverwriteType {
    Role,
    User,
}

/// Compute effective permissions for a member in a channel.
///
/// Algorithm:
/// 1. Start with @everyone role permissions
/// 2. OR in all assigned role permissions
/// 3. Apply channel overrides for @everyone
/// 4. Apply channel overrides for each role
/// 5. Apply channel overrides for the specific user
/// 6. If ADMINISTRATOR, return all permissions
pub fn compute_permissions(
    base_permissions: Permissions,
    role_permissions: &[Permissions],
    channel_overwrites: &[PermissionOverwrite],
    member_role_ids: &[uuid::Uuid],
    member_id: uuid::Uuid,
    everyone_role_id: uuid::Uuid,
) -> Permissions {
    // Step 1 & 2: Combine base + all role permissions
    let mut perms = role_permissions
        .iter()
        .fold(base_permissions, |acc, &rp| acc | rp);

    // Admin bypasses everything
    if perms.is_admin() {
        return Permissions::all();
    }

    // Step 3-5: Apply channel overwrites
    let mut allow = Permissions::empty();
    let mut deny = Permissions::empty();

    // @everyone overrides
    for ow in channel_overwrites {
        if ow.target_type == OverwriteType::Role && ow.target_id == everyone_role_id {
            allow |= Permissions::from_bits_truncate(ow.allow);
            deny |= Permissions::from_bits_truncate(ow.deny);
        }
    }

    // Role overrides
    for ow in channel_overwrites {
        if ow.target_type == OverwriteType::Role && member_role_ids.contains(&ow.target_id) {
            allow |= Permissions::from_bits_truncate(ow.allow);
            deny |= Permissions::from_bits_truncate(ow.deny);
        }
    }

    perms &= !deny;
    perms |= allow;

    // User-specific overrides (highest priority)
    for ow in channel_overwrites {
        if ow.target_type == OverwriteType::User && ow.target_id == member_id {
            perms &= !Permissions::from_bits_truncate(ow.deny);
            perms |= Permissions::from_bits_truncate(ow.allow);
        }
    }

    perms
}
