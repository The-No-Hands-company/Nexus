// Module gating and enforcement utilities
// Ensures that both client and server respect effective_enabled_modules

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NexusModule {
    /// Core messaging - DMs, group chats, channels
    Messages,
    /// Direct messaging capabilities
    Dms,
    /// Voice calls and video calls
    Calls,
    /// Contacts and presence
    Contacts,
    /// User notifications
    Notifications,
    /// Voice and audio channels
    VoiceChannels,
    /// Server directory discovery
    ServerDiscovery,
    /// Server browser and recommendations
    ServerBrowser,
    /// Bots and integrations
    Bots,
    /// Plugins and extensions
    Plugins,
    /// Community guidelines moderation tools
    Moderation,
    /// Federation and cross-instance communication
    Federation,
    /// Advanced security features (E2EE, etc.)
    AdvancedSecurity,
    /// Analytics and insights
    Analytics,
    /// Payment and monetization features
    Payments,
    /// Custom status and profiles
    Profiles,
    /// Channels for servers
    Channels,
    /// Roles and permissions management
    Roles,
    /// Invitations and access control
    Invites,
}

impl std::fmt::Display for NexusModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NexusModule::Messages => write!(f, "messages"),
            NexusModule::Dms => write!(f, "dms"),
            NexusModule::Calls => write!(f, "calls"),
            NexusModule::Contacts => write!(f, "contacts"),
            NexusModule::Notifications => write!(f, "notifications"),
            NexusModule::VoiceChannels => write!(f, "voice_channels"),
            NexusModule::ServerDiscovery => write!(f, "server_discovery"),
            NexusModule::ServerBrowser => write!(f, "server_browser"),
            NexusModule::Bots => write!(f, "bots"),
            NexusModule::Plugins => write!(f, "plugins"),
            NexusModule::Moderation => write!(f, "moderation"),
            NexusModule::Federation => write!(f, "federation"),
            NexusModule::AdvancedSecurity => write!(f, "advanced_security"),
            NexusModule::Analytics => write!(f, "analytics"),
            NexusModule::Payments => write!(f, "payments"),
            NexusModule::Profiles => write!(f, "profiles"),
            NexusModule::Channels => write!(f, "channels"),
            NexusModule::Roles => write!(f, "roles"),
            NexusModule::Invites => write!(f, "invites"),
        }
    }
}

/// Check if a module is enabled for a user
pub fn is_module_enabled(enabled_modules: &[String], module: NexusModule) -> bool {
    let module_name = module.to_string();
    enabled_modules.iter().any(|m| m == &module_name)
}

/// Filter routes/features based on enabled modules
pub fn filter_by_enabled_modules<T>(
    items: Vec<T>,
    enabled_modules: &[String],
    module_fn: impl Fn(&T) -> NexusModule,
) -> Vec<T> {
    items.into_iter()
        .filter(|item| is_module_enabled(enabled_modules, module_fn(item)))
        .collect()
}

/// Default modules for "full" mode
pub fn default_modules_full() -> Vec<String> {
    vec![
        "messages",
        "dms",
        "calls",
        "contacts",
        "notifications",
        "voice_channels",
        "server_discovery",
        "server_browser",
        "bots",
        "plugins",
        "moderation",
        "federation",
        "advanced_security",
        "analytics",
        "payments",
        "profiles",
        "channels",
        "roles",
        "invites",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Default modules for "messaging" mode (WhatsApp-like)
pub fn default_modules_messaging() -> Vec<String> {
    vec![
        "messages",
        "dms",
        "calls",
        "contacts",
        "notifications",
        "profiles",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}
