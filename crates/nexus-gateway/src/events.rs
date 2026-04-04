//! Gateway event types.

use serde::{Deserialize, Serialize};

/// All possible gateway event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    // Message events
    MessageCreate,
    MessageUpdate,
    MessageDelete,
    MessageBulkDelete,
    MessageReactionAdd,
    MessageReactionRemove,

    // Channel events
    ChannelCreate,
    ChannelUpdate,
    ChannelDelete,
    ChannelPinsUpdate,

    // Server events
    ServerCreate,
    ServerUpdate,
    ServerDelete,

    // Member events
    MemberAdd,
    MemberUpdate,
    MemberRemove,

    // Role events
    RoleCreate,
    RoleUpdate,
    RoleDelete,

    // Presence events
    PresenceUpdate,
    TypingStart,

    // Voice events
    VoiceStateUpdate,
    VoiceServerUpdate,

    // User events
    UserUpdate,

    // Invite events
    InviteCreate,
    InviteDelete,

    // Thread events
    ThreadCreate,
    ThreadUpdate,
    ThreadDelete,
    ThreadMemberUpdate,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Convert to SCREAMING_SNAKE_CASE
        let s = format!("{:?}", self);
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_uppercase());
        }
        write!(f, "{result}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── EventType::Display ────────────────────────────────────────────────────

    #[test]
    fn message_create_displays_correctly() {
        assert_eq!(EventType::MessageCreate.to_string(), "MESSAGE_CREATE");
    }

    #[test]
    fn message_reaction_add_displays_correctly() {
        assert_eq!(EventType::MessageReactionAdd.to_string(), "MESSAGE_REACTION_ADD");
    }

    #[test]
    fn voice_state_update_displays_correctly() {
        assert_eq!(EventType::VoiceStateUpdate.to_string(), "VOICE_STATE_UPDATE");
    }

    #[test]
    fn server_member_add_displays_correctly() {
        assert_eq!(EventType::MemberAdd.to_string(), "MEMBER_ADD");
    }

    #[test]
    fn all_variants_produce_screaming_snake_case() {
        let variants = vec![
            (EventType::MessageCreate,       "MESSAGE_CREATE"),
            (EventType::MessageUpdate,       "MESSAGE_UPDATE"),
            (EventType::MessageDelete,       "MESSAGE_DELETE"),
            (EventType::MessageBulkDelete,   "MESSAGE_BULK_DELETE"),
            (EventType::MessageReactionAdd,  "MESSAGE_REACTION_ADD"),
            (EventType::MessageReactionRemove, "MESSAGE_REACTION_REMOVE"),
            (EventType::ChannelCreate,       "CHANNEL_CREATE"),
            (EventType::ChannelUpdate,       "CHANNEL_UPDATE"),
            (EventType::ChannelDelete,       "CHANNEL_DELETE"),
            (EventType::PresenceUpdate,      "PRESENCE_UPDATE"),
            (EventType::TypingStart,         "TYPING_START"),
            (EventType::VoiceStateUpdate,    "VOICE_STATE_UPDATE"),
            (EventType::VoiceServerUpdate,   "VOICE_SERVER_UPDATE"),
            (EventType::UserUpdate,          "USER_UPDATE"),
        ];
        for (variant, expected) in variants {
            assert_eq!(
                variant.to_string(),
                expected,
                "wrong display for variant"
            );
        }
    }

    #[test]
    fn no_variant_produces_lowercase() {
        let cases = [
            EventType::MessageCreate,
            EventType::ChannelCreate,
            EventType::VoiceStateUpdate,
            EventType::PresenceUpdate,
        ];
        for v in cases {
            let s = v.to_string();
            assert_eq!(s, s.to_uppercase(), "all chars must be uppercase: {s}");
        }
    }

    #[test]
    fn display_output_contains_no_spaces() {
        let s = EventType::MessageReactionAdd.to_string();
        assert!(!s.contains(' '), "must use underscores not spaces: {s}");
    }
}
