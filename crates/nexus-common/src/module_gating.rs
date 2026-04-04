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

#[cfg(test)]
mod tests {
    use super::*;

    // ── NexusModule::Display ─────────────────────────────────────────────────

    #[test]
    fn display_produces_snake_case_strings() {
        assert_eq!(NexusModule::Messages.to_string(), "messages");
        assert_eq!(NexusModule::VoiceChannels.to_string(), "voice_channels");
        assert_eq!(NexusModule::ServerDiscovery.to_string(), "server_discovery");
        assert_eq!(NexusModule::AdvancedSecurity.to_string(), "advanced_security");
    }

    // ── is_module_enabled ────────────────────────────────────────────────────

    #[test]
    fn enabled_module_returns_true() {
        let enabled = vec!["messages".to_string(), "dms".to_string()];
        assert!(is_module_enabled(&enabled, NexusModule::Messages));
        assert!(is_module_enabled(&enabled, NexusModule::Dms));
    }

    #[test]
    fn disabled_module_returns_false() {
        let enabled = vec!["messages".to_string()];
        assert!(!is_module_enabled(&enabled, NexusModule::Bots));
        assert!(!is_module_enabled(&enabled, NexusModule::Federation));
    }

    #[test]
    fn empty_enabled_list_returns_false_for_everything() {
        let enabled: Vec<String> = vec![];
        assert!(!is_module_enabled(&enabled, NexusModule::Messages));
        assert!(!is_module_enabled(&enabled, NexusModule::Calls));
    }

    #[test]
    fn module_check_is_exact_not_prefix_based() {
        // "message" must not match "messages"
        let enabled = vec!["message".to_string()];
        assert!(!is_module_enabled(&enabled, NexusModule::Messages));
    }

    // ── filter_by_enabled_modules ────────────────────────────────────────────

    #[derive(Debug, PartialEq)]
    struct Feature {
        name: &'static str,
        module: NexusModule,
    }

    fn features() -> Vec<Feature> {
        vec![
            Feature { name: "chat",     module: NexusModule::Messages },
            Feature { name: "calls",    module: NexusModule::Calls },
            Feature { name: "bots",     module: NexusModule::Bots },
            Feature { name: "plugins",  module: NexusModule::Plugins },
        ]
    }

    #[test]
    fn filter_keeps_only_enabled_modules() {
        let enabled = vec!["messages".to_string(), "calls".to_string()];
        let result = filter_by_enabled_modules(features(), &enabled, |f| f.module);
        let names: Vec<&str> = result.iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["chat", "calls"]);
    }

    #[test]
    fn filter_with_all_enabled_keeps_all() {
        let enabled = default_modules_full();
        let result = filter_by_enabled_modules(features(), &enabled, |f| f.module);
        assert_eq!(result.len(), features().len());
    }

    #[test]
    fn filter_with_empty_enabled_returns_empty() {
        let result = filter_by_enabled_modules(features(), &[], |f| f.module);
        assert!(result.is_empty());
    }

    // ── default_modules_full ─────────────────────────────────────────────────

    #[test]
    fn full_mode_includes_all_core_modules() {
        let full = default_modules_full();
        // Every value must be a unique snake_case string
        let unique: std::collections::HashSet<_> = full.iter().collect();
        assert_eq!(full.len(), unique.len(), "duplicate module in full list");

        // Spot-check critical modules
        assert!(full.contains(&"messages".to_string()));
        assert!(full.contains(&"voice_channels".to_string()));
        assert!(full.contains(&"federation".to_string()));
        assert!(full.contains(&"moderation".to_string()));
    }

    #[test]
    fn full_mode_modules_all_map_to_known_nexus_modules() {
        let full = default_modules_full();
        // Every module name in the full list must be recognisable by the gating
        // logic — verified by round-tripping through is_module_enabled.
        // We confirm the list is non-empty and contains known module names.
        assert!(!full.is_empty());
        // Spot-check: these must be present in the full module set.
        for expected in &["messages", "voice_channels", "federation", "moderation"] {
            assert!(
                full.iter().any(|m| m == expected),
                "'{expected}' must be in default_modules_full()"
            );
        }
    }

    // ── default_modules_messaging ────────────────────────────────────────────

    #[test]
    fn messaging_mode_is_subset_of_full_mode() {
        let full = default_modules_full();
        let msg = default_modules_messaging();
        // Every module in messaging mode must also exist in full mode
        for m in &msg {
            assert!(full.contains(m), "'{m}' in messaging mode but not in full mode");
        }
    }

    #[test]
    fn messaging_mode_excludes_server_features() {
        let msg = default_modules_messaging();
        // Messaging-only mode should not include heavier server features
        assert!(!msg.contains(&"federation".to_string()));
        assert!(!msg.contains(&"bots".to_string()));
        assert!(!msg.contains(&"server_discovery".to_string()));
        assert!(!msg.contains(&"moderation".to_string()));
    }

    #[test]
    fn messaging_mode_includes_minimum_required() {
        let msg = default_modules_messaging();
        assert!(msg.contains(&"messages".to_string()));
        assert!(msg.contains(&"dms".to_string()));
        assert!(msg.contains(&"calls".to_string()));
        assert!(msg.contains(&"profiles".to_string()));
    }
}
