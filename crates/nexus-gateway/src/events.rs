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
