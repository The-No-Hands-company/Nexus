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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    // ── Permissions bitfield ─────────────────────────────────────────────────

    #[test]
    fn empty_permissions_contains_nothing() {
        let p = Permissions::empty();
        assert!(!p.contains(Permissions::SEND_MESSAGES));
        assert!(!p.contains(Permissions::ADMINISTRATOR));
        assert!(!p.is_admin());
    }

    #[test]
    fn individual_bits_are_distinct() {
        // Every defined permission bit must be unique (no aliasing).
        let all_bits: Vec<i64> = vec![
            Permissions::VIEW_CHANNEL.bits(),
            Permissions::MANAGE_SERVER.bits(),
            Permissions::MANAGE_CHANNELS.bits(),
            Permissions::MANAGE_ROLES.bits(),
            Permissions::CREATE_INVITES.bits(),
            Permissions::KICK_MEMBERS.bits(),
            Permissions::BAN_MEMBERS.bits(),
            Permissions::SEND_MESSAGES.bits(),
            Permissions::MUTE_MEMBERS.bits(),
            Permissions::ADMINISTRATOR.bits(),
        ];
        let unique: std::collections::HashSet<_> = all_bits.iter().collect();
        assert_eq!(all_bits.len(), unique.len(), "duplicate permission bits detected");
    }

    #[test]
    fn or_combines_permissions() {
        let p = Permissions::SEND_MESSAGES | Permissions::VIEW_CHANNEL;
        assert!(p.contains(Permissions::SEND_MESSAGES));
        assert!(p.contains(Permissions::VIEW_CHANNEL));
        assert!(!p.contains(Permissions::ADMINISTRATOR));
    }

    #[test]
    fn and_not_removes_permission() {
        let p = Permissions::SEND_MESSAGES | Permissions::VIEW_CHANNEL;
        let stripped = p & !Permissions::SEND_MESSAGES;
        assert!(!stripped.contains(Permissions::SEND_MESSAGES));
        assert!(stripped.contains(Permissions::VIEW_CHANNEL));
    }

    #[test]
    fn administrator_implies_all_via_has() {
        let p = Permissions::ADMINISTRATOR;
        assert!(p.is_admin());
        // has() returns true for any permission when ADMINISTRATOR is set
        assert!(p.has(Permissions::SEND_MESSAGES));
        assert!(p.has(Permissions::BAN_MEMBERS));
        assert!(p.has(Permissions::MANAGE_SERVER));
        assert!(p.has(Permissions::MUTE_MEMBERS));
    }

    #[test]
    fn has_returns_false_without_permission() {
        let p = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
        assert!(!p.has(Permissions::BAN_MEMBERS));
        assert!(!p.has(Permissions::ADMINISTRATOR));
    }

    #[test]
    fn has_returns_true_for_held_permission() {
        let p = Permissions::BAN_MEMBERS | Permissions::KICK_MEMBERS;
        assert!(p.has(Permissions::BAN_MEMBERS));
        assert!(p.has(Permissions::KICK_MEMBERS));
    }

    #[test]
    fn default_everyone_contains_expected_permissions() {
        let p = Permissions::default_everyone();
        assert!(p.contains(Permissions::VIEW_CHANNEL));
        assert!(p.contains(Permissions::SEND_MESSAGES));
        assert!(p.contains(Permissions::ADD_REACTIONS));
        assert!(p.contains(Permissions::CONNECT));
        assert!(p.contains(Permissions::SPEAK));
        // Should NOT grant moderation powers
        assert!(!p.contains(Permissions::BAN_MEMBERS));
        assert!(!p.contains(Permissions::KICK_MEMBERS));
        assert!(!p.contains(Permissions::ADMINISTRATOR));
        assert!(!p.contains(Permissions::MANAGE_SERVER));
    }

    #[test]
    fn from_bits_truncate_ignores_unknown_bits() {
        // Bits beyond the defined flags should be masked out silently.
        let p = Permissions::from_bits_truncate(i64::MAX);
        // All known bits are set but no panic or invalid state.
        assert!(p.contains(Permissions::ADMINISTRATOR));
        assert!(p.contains(Permissions::SEND_MESSAGES));
    }

    // ── compute_permissions ──────────────────────────────────────────────────

    fn setup() -> (Uuid, Uuid, Uuid) {
        let everyone_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        (everyone_id, member_id, role_id)
    }

    #[test]
    fn base_permissions_without_overwrites() {
        let (everyone_id, member_id, _) = setup();
        let base = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;

        let effective = compute_permissions(
            base,
            &[],
            &[],
            &[],
            member_id,
            everyone_id,
        );

        assert!(effective.contains(Permissions::VIEW_CHANNEL));
        assert!(effective.contains(Permissions::SEND_MESSAGES));
        assert!(!effective.contains(Permissions::BAN_MEMBERS));
    }

    #[test]
    fn role_permissions_are_ored_in() {
        let (everyone_id, member_id, role_id) = setup();
        let base = Permissions::VIEW_CHANNEL;
        let role_perms = Permissions::BAN_MEMBERS | Permissions::KICK_MEMBERS;

        let effective = compute_permissions(
            base,
            &[role_perms],
            &[],
            &[role_id],
            member_id,
            everyone_id,
        );

        assert!(effective.contains(Permissions::VIEW_CHANNEL));
        assert!(effective.contains(Permissions::BAN_MEMBERS));
        assert!(effective.contains(Permissions::KICK_MEMBERS));
    }

    #[test]
    fn channel_deny_overwrite_removes_permission() {
        let (everyone_id, member_id, _) = setup();
        let base = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;

        let overwrite = PermissionOverwrite {
            target_id: everyone_id,
            target_type: OverwriteType::Role,
            allow: 0,
            deny: Permissions::SEND_MESSAGES.bits(),
        };

        let effective = compute_permissions(
            base,
            &[],
            &[overwrite],
            &[],
            member_id,
            everyone_id,
        );

        assert!(effective.contains(Permissions::VIEW_CHANNEL));
        assert!(!effective.contains(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn channel_allow_overwrite_grants_permission() {
        let (everyone_id, member_id, _) = setup();
        // Base has no MANAGE_MESSAGES
        let base = Permissions::VIEW_CHANNEL;

        let overwrite = PermissionOverwrite {
            target_id: everyone_id,
            target_type: OverwriteType::Role,
            allow: Permissions::MANAGE_MESSAGES.bits(),
            deny: 0,
        };

        let effective = compute_permissions(
            base,
            &[],
            &[overwrite],
            &[],
            member_id,
            everyone_id,
        );

        assert!(effective.contains(Permissions::MANAGE_MESSAGES));
    }

    #[test]
    fn user_overwrite_takes_priority_over_role_deny() {
        let (everyone_id, member_id, _) = setup();
        let base = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;

        // Everyone deny SEND_MESSAGES
        let role_deny = PermissionOverwrite {
            target_id: everyone_id,
            target_type: OverwriteType::Role,
            allow: 0,
            deny: Permissions::SEND_MESSAGES.bits(),
        };
        // But user-level allow restores it
        let user_allow = PermissionOverwrite {
            target_id: member_id,
            target_type: OverwriteType::User,
            allow: Permissions::SEND_MESSAGES.bits(),
            deny: 0,
        };

        let effective = compute_permissions(
            base,
            &[],
            &[role_deny, user_allow],
            &[],
            member_id,
            everyone_id,
        );

        assert!(effective.contains(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn user_deny_overwrite_removes_even_role_granted_permission() {
        let (everyone_id, member_id, role_id) = setup();
        let base = Permissions::VIEW_CHANNEL;
        let role_perms = Permissions::SEND_MESSAGES;

        // Role grants SEND_MESSAGES, but user override denies it
        let user_deny = PermissionOverwrite {
            target_id: member_id,
            target_type: OverwriteType::User,
            allow: 0,
            deny: Permissions::SEND_MESSAGES.bits(),
        };

        let effective = compute_permissions(
            base,
            &[role_perms],
            &[user_deny],
            &[role_id],
            member_id,
            everyone_id,
        );

        assert!(!effective.contains(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn administrator_bypasses_channel_deny_overwrites() {
        let (everyone_id, member_id, _) = setup();
        let base = Permissions::ADMINISTRATOR;

        let deny_all = PermissionOverwrite {
            target_id: everyone_id,
            target_type: OverwriteType::Role,
            allow: 0,
            deny: i64::MAX, // deny everything
        };

        let effective = compute_permissions(
            base,
            &[],
            &[deny_all],
            &[],
            member_id,
            everyone_id,
        );

        // Admin short-circuits before channel overwrites are applied
        assert!(effective.contains(Permissions::SEND_MESSAGES));
        assert!(effective.contains(Permissions::BAN_MEMBERS));
        assert!(effective.contains(Permissions::VIEW_CHANNEL));
    }

    #[test]
    fn role_overwrites_only_apply_to_members_holding_the_role() {
        let (everyone_id, member_id, role_id) = setup();
        let other_role_id = Uuid::new_v4();
        let base = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;

        // Overwrite targets a role the member does NOT hold
        let overwrite = PermissionOverwrite {
            target_id: other_role_id,
            target_type: OverwriteType::Role,
            allow: 0,
            deny: Permissions::SEND_MESSAGES.bits(),
        };

        let effective = compute_permissions(
            base,
            &[],
            &[overwrite],
            &[role_id], // member holds role_id, not other_role_id
            member_id,
            everyone_id,
        );

        // Deny from a role the member doesn't have must NOT apply
        assert!(effective.contains(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn multiple_role_permissions_are_all_included() {
        let (everyone_id, member_id, _) = setup();
        let role1_id = Uuid::new_v4();
        let role2_id = Uuid::new_v4();

        let effective = compute_permissions(
            Permissions::empty(),
            &[Permissions::VIEW_CHANNEL, Permissions::BAN_MEMBERS],
            &[],
            &[role1_id, role2_id],
            member_id,
            everyone_id,
        );

        assert!(effective.contains(Permissions::VIEW_CHANNEL));
        assert!(effective.contains(Permissions::BAN_MEMBERS));
    }
}
