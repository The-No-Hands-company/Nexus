//! Manual `sqlx::FromRow<'_, sqlx::any::AnyRow>` implementations for all
//! nexus-common model types.
//!
//! `sqlx::AnyPool` only decodes primitive types natively (i64, f64, bool,
//! String, bytes).  UUID and DateTime columns must be decoded as `String` and
//! then parsed.  JSON/array columns are stored as JSON text.
//!
//! **Why manual instead of `#[derive(sqlx::FromRow)]`?**
//! The derive macro generates a *blanket* `impl<DB>` with trait bounds.  Rust's
//! coherence checker rejects a manual `impl<AnyRow>` alongside that blanket even
//! when the bounds are never satisfied for `Any`, because a future downstream
//! crate might add the missing impls.  Removing the derive from the struct and
//! writing three specific impls (PgRow / SqliteRow / AnyRow) sidesteps the
//! conflict entirely.  Since the codebase now uses AnyPool exclusively we only
//! need the AnyRow impl.

#![allow(clippy::cast_possible_truncation)]

use chrono::{DateTime, Utc};
use sqlx::{Row, any::AnyRow};
use uuid::Uuid;

use crate::models::{
    accessibility::{
        MessageTranslation, MessageTtsRequest, UserAccessibilitySettings, VoiceCaption,
    },
    ai_intelligence::{
        AiAuditEntry, AiConsent, AiSuggestion, RaidDetection, SearchEmbedding, SearchQuery,
        ThreadSummary, ToxicityScore, VoiceCommand, VoiceTranscript,
    },
    channel::{Channel, ChannelType},
    collaboration::{
        AiPreferences, CalendarEvent, CalendarRsvp, ChannelDigest, ChecklistItem, FileVersion,
        ServerStorageQuota, Task, TaskReminder,
    },
    crypto::{
        Device, DeviceType, DeviceVerification, E2eeChannel, E2eeSession, EncryptedMessage,
        OneTimePreKey, VerificationMethod,
    },
    ecosystem::{
        BulkInvitation, CreatorVetting, IdentityLevel, ImportJob, MarketplaceMonetization,
        MarketplacePlugin, OnboardingProgress, PluginInstall, PluginReview,
        ServerAnalyticsSnapshot, ServerTemplate,
    },
    growth::{
        Achievement, ActivityStreak, ClipboardSync, DeviceSession, GamificationConfig,
        OfflineQueueItem, OnboardingFlow, ServerRecommendation, SyncCursor, UserAchievement,
        UserXp,
    },
    member::Member,
    phantom::PhantomIdentity,
    multimedia::{
        Drawing, MediaGalleryFilter, Story, StoryView, VideoNote, VoiceMusicQueueItem, VoiceNote,
        VoiceSettings,
    },
    relationship::{Relationship, RelationshipStatus},
    rich::{AttachmentRow, ServerEmojiRow, ThreadRow},
    role::Role,
    scalability::{
        FederationDedupEntry, FederationEventBatch, FederationRoute, MemberPruneRule,
        ScalingConfig, ScalingMetric, SfuNode, SlowModeOverride, UpgradeRecord, VoiceQualityLog,
    },
    server::{Invite, Server},
    sustainability::{
        CapabilityNegotiation, ContributorBadge, GovernancePoll, GovernanceProposal,
        MigrationGuide, PollVote, ProtocolVersion, SecurityAudit, TutorialProgress,
        VulnerabilityRecord,
    },
    user::{User, UserPresence},
    voice_collab::{
        BreakoutRoom, CollabSession, LiveStream, SpatialAudioConfig, StreamViewer, VideoLayout,
        VirtualBackground, VoicePreset,
    },
};

// ── Internal helpers ──────────────────────────────────────────────────────────

fn uuid(row: &AnyRow, col: &str) -> Result<Uuid, sqlx::Error> {
    let s: String = row.try_get(col)?;
    Uuid::parse_str(&s).map_err(|e| sqlx::Error::Decode(Box::new(e) as _))
}

fn opt_uuid(row: &AnyRow, col: &str) -> Result<Option<Uuid>, sqlx::Error> {
    let s: Option<String> = row.try_get(col)?;
    s.map(|v| Uuid::parse_str(&v).map_err(|e| sqlx::Error::Decode(Box::new(e) as _)))
        .transpose()
}

fn dt(row: &AnyRow, col: &str) -> Result<DateTime<Utc>, sqlx::Error> {
    let s: String = row.try_get(col)?;
    parse_dt(&s).map_err(sqlx::Error::Decode)
}

fn opt_dt(row: &AnyRow, col: &str) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    let s: Option<String> = row.try_get(col)?;
    s.map(|v| parse_dt(&v).map_err(sqlx::Error::Decode))
        .transpose()
}

fn parse_dt(s: &str) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // RFC 3339 / ISO 8601 (e.g. "2024-01-15T10:30:00+00:00")
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Ok(d.with_timezone(&Utc));
    }
    // Normalize Postgres TIMESTAMPTZ::text: "2026-02-22 13:04:47.779907+00"
    // Replace first space with T, then pad short tz offset "+HH" → "+HH:00".
    let iso = s.replacen(' ', "T", 1);
    let iso = {
        let len = iso.len();
        if len >= 3 {
            let last3 = &iso[len - 3..];
            if (last3.starts_with('+') || last3.starts_with('-'))
                && last3[1..].chars().all(|c| c.is_ascii_digit())
            {
                format!("{iso}:00")
            } else {
                iso
            }
        } else {
            iso
        }
    };
    if let Ok(d) = DateTime::parse_from_rfc3339(&iso) {
        return Ok(d.with_timezone(&Utc));
    }
    // SQLite CURRENT_TIMESTAMP: "2024-01-15 10:30:00"
    if let Ok(d) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(d.and_utc());
    }
    // SQLite with fractional seconds: "2024-01-15 10:30:00.123456"
    if let Ok(d) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
        return Ok(d.and_utc());
    }
    Err(format!("cannot parse timestamp '{s}'").into())
}

fn json(row: &AnyRow, col: &str) -> Result<serde_json::Value, sqlx::Error> {
    let s: String = row.try_get(col)?;
    serde_json::from_str(&s).map_err(|e| sqlx::Error::Decode(Box::new(e) as _))
}

fn uuid_vec(row: &AnyRow, col: &str) -> Result<Vec<Uuid>, sqlx::Error> {
    let s: String = row.try_get(col)?;
    if s.trim() == "[]" || s.is_empty() {
        return Ok(vec![]);
    }
    let strs: Vec<String> =
        serde_json::from_str(&s).map_err(|e| sqlx::Error::Decode(Box::new(e) as _))?;
    strs.iter()
        .map(|v| Uuid::parse_str(v).map_err(|e| sqlx::Error::Decode(Box::new(e) as _)))
        .collect()
}

fn str_vec(row: &AnyRow, col: &str) -> Result<Vec<String>, sqlx::Error> {
    let s: String = row.try_get(col)?;
    if s.trim() == "[]" || s.is_empty() {
        return Ok(vec![]);
    }
    serde_json::from_str(&s).map_err(|e| sqlx::Error::Decode(Box::new(e) as _))
}

fn parse_enum<T>(row: &AnyRow, col: &str, f: impl Fn(&str) -> Option<T>) -> Result<T, sqlx::Error> {
    let s: String = row.try_get(col)?;
    f(&s).ok_or_else(|| sqlx::Error::Decode(format!("unknown enum variant: {s}").into()))
}

// ── User ──────────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for User {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(User {
            id: uuid(row, "id")?,
            username: row.try_get("username")?,
            display_name: row.try_get("display_name")?,
            email: row.try_get("email")?,
            password_hash: row.try_get("password_hash")?,
            avatar: row.try_get("avatar")?,
            banner: row.try_get("banner")?,
            bio: row.try_get("bio")?,
            status: row.try_get("status")?,
            presence: parse_enum(row, "presence", |s| match s {
                "online" => Some(UserPresence::Online),
                "idle" => Some(UserPresence::Idle),
                "do_not_disturb" => Some(UserPresence::DoNotDisturb),
                "invisible" => Some(UserPresence::Invisible),
                _ => Some(UserPresence::Offline),
            })?,
            flags: row.try_get("flags")?,
            totp_enabled: row.try_get("totp_enabled").unwrap_or(false),
            server_name: row.try_get("server_name").unwrap_or(None),
            is_remote: row.try_get("is_remote").unwrap_or(false),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── Server ────────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for Server {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(Server {
            id: uuid(row, "id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            icon: row.try_get("icon")?,
            banner: row.try_get("banner")?,
            owner_id: uuid(row, "owner_id")?,
            region: row.try_get("region")?,
            is_public: row.try_get("is_public")?,
            features: json(row, "features")?,
            settings: json(row, "settings")?,
            vanity_code: row.try_get("vanity_code")?,
            member_count: row.try_get("member_count")?,
            max_file_size: row.try_get("max_file_size")?,
            require_2fa: row.try_get("require_2fa").unwrap_or(false),
            spam_window_secs: row.try_get("spam_window_secs").unwrap_or(30),
            spam_max_messages: row.try_get("spam_max_messages").unwrap_or(3),
            boost_tier: row.try_get("boost_tier").unwrap_or(0),
            booster_count: row.try_get("booster_count").unwrap_or(0),
            tags: str_vec(row, "tags").unwrap_or_default(),
            category: row.try_get("category").unwrap_or(None),
            activity_score: row.try_get("activity_score").unwrap_or(0),
            featured_at: opt_dt(row, "featured_at").unwrap_or(None),
            tip_jar_url: row.try_get("tip_jar_url").unwrap_or(None),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── Channel ───────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for Channel {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(Channel {
            id: uuid(row, "id")?,
            server_id: opt_uuid(row, "server_id")?,
            parent_id: opt_uuid(row, "parent_id")?,
            channel_type: parse_enum(row, "channel_type", |s| match s {
                "text" => Some(ChannelType::Text),
                "voice" => Some(ChannelType::Voice),
                "category" => Some(ChannelType::Category),
                "announcement" => Some(ChannelType::Announcement),
                "forum" => Some(ChannelType::Forum),
                "dm" => Some(ChannelType::Dm),
                "group_dm" => Some(ChannelType::GroupDm),
                "stage" => Some(ChannelType::Stage),
                "thread" => Some(ChannelType::Thread),
                _ => None,
            })?,
            name: row.try_get("name")?,
            topic: row.try_get("topic")?,
            position: row.try_get("position")?,
            nsfw: row.try_get("nsfw")?,
            rate_limit_per_user: row.try_get("rate_limit_per_user")?,
            bitrate: row.try_get("bitrate")?,
            user_limit: row.try_get("user_limit")?,
            encrypted: row.try_get("encrypted")?,
            permission_overwrites: json(row, "permission_overwrites")?,
            last_message_id: opt_uuid(row, "last_message_id")?,
            auto_archive_duration: row.try_get("auto_archive_duration")?,
            archived: row.try_get("archived")?,
            locked: row.try_get("locked")?,
            disappear_after_seconds: row.try_get("disappear_after_seconds").unwrap_or(0),
            is_stream: row.try_get("is_stream").unwrap_or(false),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── Member ────────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for Member {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(Member {
            user_id: uuid(row, "user_id")?,
            server_id: uuid(row, "server_id")?,
            nickname: row.try_get("nickname")?,
            avatar: row.try_get("avatar")?,
            roles: uuid_vec(row, "roles")?,
            muted: row.try_get("muted")?,
            deafened: row.try_get("deafened")?,
            joined_at: dt(row, "joined_at")?,
            communication_disabled_until: opt_dt(row, "communication_disabled_until")?,
        })
    }
}

// ── Role ──────────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for Role {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(Role {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            name: row.try_get("name")?,
            color: row.try_get("color")?,
            hoist: row.try_get("hoist")?,
            icon: row.try_get("icon")?,
            position: row.try_get("position")?,
            permissions: row.try_get("permissions")?,
            mentionable: row.try_get("mentionable")?,
            is_default: row.try_get("is_default")?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── Device ────────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for Device {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(Device {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            name: row.try_get("name")?,
            identity_key: row.try_get("identity_key")?,
            signed_pre_key: row.try_get("signed_pre_key")?,
            signed_pre_key_sig: row.try_get("signed_pre_key_sig")?,
            signed_pre_key_id: row.try_get("signed_pre_key_id")?,
            // opt_dt, not dt: rows created before the column existed have no
            // value, and a device that predates it should read as "rotation
            // age unknown" rather than fail to load at all.
            signed_pre_key_rotated_at: opt_dt(row, "signed_pre_key_rotated_at")?,
            device_type: parse_enum(row, "device_type", |s| match s {
                "desktop" => Some(DeviceType::Desktop),
                "mobile" => Some(DeviceType::Mobile),
                "browser" => Some(DeviceType::Browser),
                _ => Some(DeviceType::Unknown),
            })?,
            last_seen_at: opt_dt(row, "last_seen_at")?,
            verified: row.try_get("verified")?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── OneTimePreKey ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for OneTimePreKey {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(OneTimePreKey {
            id: uuid(row, "id")?,
            device_id: uuid(row, "device_id")?,
            key_id: row.try_get("key_id")?,
            public_key: row.try_get("public_key")?,
            consumed: row.try_get("consumed")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── E2eeSession ───────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for E2eeSession {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(E2eeSession {
            id: uuid(row, "id")?,
            owner_device_id: uuid(row, "owner_device_id")?,
            remote_device_id: uuid(row, "remote_device_id")?,
            session_state: row.try_get("session_state")?,
            ratchet_step: row.try_get("ratchet_step")?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── ThreadRow ─────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ThreadRow {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ThreadRow {
            channel_id: uuid(row, "channel_id")?,
            parent_message_id: opt_uuid(row, "parent_message_id")?,
            owner_id: uuid(row, "owner_id")?,
            title: row.try_get("title")?,
            message_count: row.try_get("message_count")?,
            member_count: row.try_get("member_count")?,
            auto_archive_minutes: row.try_get("auto_archive_minutes")?,
            archived: row.try_get("archived")?,
            archived_at: opt_dt(row, "archived_at")?,
            locked: row.try_get("locked")?,
            tags: str_vec(row, "tags")?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
            parent_channel_id: opt_uuid(row, "parent_channel_id")?,
        })
    }
}

// ── ServerEmojiRow ────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ServerEmojiRow {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ServerEmojiRow {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            creator_id: opt_uuid(row, "creator_id")?,
            name: row.try_get("name")?,
            storage_key: row.try_get("storage_key")?,
            url: row.try_get("url")?,
            animated: row.try_get("animated")?,
            managed: row.try_get("managed")?,
            available: row.try_get("available")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── AttachmentRow ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for AttachmentRow {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(AttachmentRow {
            id: uuid(row, "id")?,
            uploader_id: uuid(row, "uploader_id")?,
            server_id: opt_uuid(row, "server_id")?,
            channel_id: opt_uuid(row, "channel_id")?,
            message_id: opt_uuid(row, "message_id")?,
            filename: row.try_get("filename")?,
            content_type: row.try_get("content_type")?,
            size: row.try_get("size")?,
            storage_key: row.try_get("storage_key")?,
            url: row.try_get("url")?,
            width: row.try_get("width")?,
            height: row.try_get("height")?,
            duration_secs: row.try_get("duration_secs")?,
            spoiler: row.try_get("spoiler")?,
            blurhash: row.try_get("blurhash")?,
            sha256: row.try_get("sha256")?,
            status: row.try_get("status")?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── DeviceVerification ────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for DeviceVerification {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(DeviceVerification {
            id: uuid(row, "id")?,
            verifier_id: uuid(row, "verifier_id")?,
            target_device_id: uuid(row, "target_device_id")?,
            method: parse_enum(row, "method", |s| match s {
                "safety_number" => Some(VerificationMethod::SafetyNumber),
                "qr_scan" => Some(VerificationMethod::QrScan),
                "emoji" => Some(VerificationMethod::Emoji),
                _ => None,
            })?,
            verified_at: dt(row, "verified_at")?,
        })
    }
}

// ── E2eeChannel ───────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for E2eeChannel {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(E2eeChannel {
            channel_id: uuid(row, "channel_id")?,
            enabled_by: uuid(row, "enabled_by")?,
            enabled_at: dt(row, "enabled_at")?,
            rotation_interval_secs: row.try_get("rotation_interval_secs")?,
            last_rotated_at: dt(row, "last_rotated_at")?,
        })
    }
}

// ── EncryptedMessage ──────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for EncryptedMessage {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        let ciphertext_map_str: String = row.try_get("ciphertext_map")?;
        let ciphertext_map = serde_json::from_str(&ciphertext_map_str)
            .map_err(|e| sqlx::Error::Decode(Box::new(e) as _))?;
        let attachment_meta = row
            .try_get::<Option<String>, _>("attachment_meta")?
            .map(|s| serde_json::from_str::<serde_json::Value>(&s))
            .transpose()
            .map_err(|e| sqlx::Error::Decode(Box::new(e) as _))?;
        Ok(EncryptedMessage {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            sender_id: uuid(row, "sender_id")?,
            sender_device_id: uuid(row, "sender_device_id")?,
            ciphertext_map,
            attachment_meta,
            sequence: row.try_get("sequence")?,
            sender_ratchet_step: row.try_get("sender_ratchet_step")?,
            client_ts: opt_dt(row, "client_ts")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── Invite ────────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for Invite {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(Invite {
            code: row.try_get("code")?,
            server_id: uuid(row, "server_id")?,
            channel_id: opt_uuid(row, "channel_id")?,
            inviter_id: uuid(row, "inviter_id")?,
            max_uses: row.try_get("max_uses")?,
            uses: row.try_get("uses")?,
            expires_at: opt_dt(row, "expires_at")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── Relationship ──────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for Relationship {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(Relationship {
            id: uuid(row, "id")?,
            requester_id: uuid(row, "requester_id")?,
            addressee_id: uuid(row, "addressee_id")?,
            status: parse_enum(row, "status", |s| match s {
                "pending" => Some(RelationshipStatus::Pending),
                "accepted" => Some(RelationshipStatus::Accepted),
                "blocked" => Some(RelationshipStatus::Blocked),
                _ => None,
            })?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── Task ──────────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for Task {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(Task {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            channel_id: uuid(row, "channel_id")?,
            creator_id: uuid(row, "creator_id")?,
            assignee_id: opt_uuid(row, "assignee_id")?,
            title: row.try_get("title")?,
            description: row.try_get("description").unwrap_or(None),
            status: row.try_get("status")?,
            priority: row.try_get("priority")?,
            due_at: opt_dt(row, "due_at").unwrap_or(None),
            completed_at: opt_dt(row, "completed_at").unwrap_or(None),
            position: row.try_get("position").unwrap_or(0),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

impl<'r> sqlx::FromRow<'r, AnyRow> for ChecklistItem {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ChecklistItem {
            id: uuid(row, "id")?,
            task_id: uuid(row, "task_id")?,
            content: row.try_get("content")?,
            checked: row.try_get("checked").unwrap_or(false),
            position: row.try_get("position").unwrap_or(0),
            created_at: dt(row, "created_at")?,
        })
    }
}

impl<'r> sqlx::FromRow<'r, AnyRow> for TaskReminder {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(TaskReminder {
            id: uuid(row, "id")?,
            task_id: uuid(row, "task_id")?,
            user_id: uuid(row, "user_id")?,
            remind_at: dt(row, "remind_at")?,
            fired: row.try_get("fired").unwrap_or(false),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── CalendarEvent ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for CalendarEvent {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(CalendarEvent {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            channel_id: opt_uuid(row, "channel_id")?,
            creator_id: uuid(row, "creator_id")?,
            title: row.try_get("title")?,
            description: row.try_get("description").unwrap_or(None),
            location: row.try_get("location").unwrap_or(None),
            starts_at: dt(row, "starts_at")?,
            ends_at: dt(row, "ends_at")?,
            all_day: row.try_get("all_day").unwrap_or(false),
            rrule: row.try_get("rrule").unwrap_or(None),
            color: row.try_get("color").unwrap_or(None),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

impl<'r> sqlx::FromRow<'r, AnyRow> for CalendarRsvp {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(CalendarRsvp {
            event_id: uuid(row, "event_id")?,
            user_id: uuid(row, "user_id")?,
            status: row.try_get("status")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── FileVersion ───────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for FileVersion {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(FileVersion {
            id: uuid(row, "id")?,
            attachment_id: uuid(row, "attachment_id")?,
            uploader_id: uuid(row, "uploader_id")?,
            version_number: row.try_get("version_number").unwrap_or(1),
            filename: row.try_get("filename")?,
            content_type: row.try_get("content_type").unwrap_or(None),
            size: row
                .try_get::<i64, _>("size")
                .or_else(|_| row.try_get::<i32, _>("size").map(i64::from))
                .unwrap_or(0),
            storage_key: row.try_get("storage_key")?,
            sha256: row.try_get("sha256").unwrap_or(None),
            comment: row.try_get("comment").unwrap_or(None),
            created_at: dt(row, "created_at")?,
        })
    }
}

impl<'r> sqlx::FromRow<'r, AnyRow> for ServerStorageQuota {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ServerStorageQuota {
            server_id: uuid(row, "server_id")?,
            max_bytes: row
                .try_get::<i64, _>("max_bytes")
                .or_else(|_| row.try_get::<i32, _>("max_bytes").map(i64::from))
                .unwrap_or(0),
            used_bytes: row
                .try_get::<i64, _>("used_bytes")
                .or_else(|_| row.try_get::<i32, _>("used_bytes").map(i64::from))
                .unwrap_or(0),
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── AiPreferences ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for AiPreferences {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(AiPreferences {
            user_id: uuid(row, "user_id")?,
            summaries_enabled: row.try_get("summaries_enabled").unwrap_or(false),
            smart_replies: row.try_get("smart_replies").unwrap_or(false),
            auto_mod_suggest: row.try_get("auto_mod_suggest").unwrap_or(false),
            digest_enabled: row.try_get("digest_enabled").unwrap_or(false),
            digest_interval: row
                .try_get("digest_interval")
                .unwrap_or_else(|_| "daily".to_string()),
            updated_at: dt(row, "updated_at")?,
        })
    }
}

impl<'r> sqlx::FromRow<'r, AnyRow> for ChannelDigest {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ChannelDigest {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            user_id: uuid(row, "user_id")?,
            period_start: dt(row, "period_start")?,
            period_end: dt(row, "period_end")?,
            summary: row.try_get("summary")?,
            message_count: row.try_get("message_count").unwrap_or(0),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Phase 17 — Multimedia & Expression
// ═══════════════════════════════════════════════════════════════════════════════

// ── VoiceNote ─────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for VoiceNote {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(VoiceNote {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            author_id: uuid(row, "author_id")?,
            storage_key: row.try_get("storage_key")?,
            filename: row.try_get("filename")?,
            content_type: row.try_get("content_type")?,
            size: row
                .try_get::<i64, _>("size")
                .or_else(|_| row.try_get::<i32, _>("size").map(i64::from))?,
            duration_ms: row.try_get("duration_ms")?,
            waveform: row.try_get::<Option<String>, _>("waveform").unwrap_or(None),
            transcript: row
                .try_get::<Option<String>, _>("transcript")
                .unwrap_or(None),
            message_id: opt_uuid(row, "message_id")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── VideoNote ─────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for VideoNote {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(VideoNote {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            author_id: uuid(row, "author_id")?,
            storage_key: row.try_get("storage_key")?,
            filename: row.try_get("filename")?,
            content_type: row.try_get("content_type")?,
            size: row
                .try_get::<i64, _>("size")
                .or_else(|_| row.try_get::<i32, _>("size").map(i64::from))?,
            duration_ms: row.try_get("duration_ms")?,
            width: row.try_get::<Option<i32>, _>("width").unwrap_or(None),
            height: row.try_get::<Option<i32>, _>("height").unwrap_or(None),
            thumbnail_key: row
                .try_get::<Option<String>, _>("thumbnail_key")
                .unwrap_or(None),
            transcript: row
                .try_get::<Option<String>, _>("transcript")
                .unwrap_or(None),
            message_id: opt_uuid(row, "message_id")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── Story ─────────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for Story {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(Story {
            id: uuid(row, "id")?,
            author_id: uuid(row, "author_id")?,
            media_type: row.try_get("media_type")?,
            storage_key: row
                .try_get::<Option<String>, _>("storage_key")
                .unwrap_or(None),
            text_content: row
                .try_get::<Option<String>, _>("text_content")
                .unwrap_or(None),
            text_style: json(row, "text_style").ok(),
            expires_at: dt(row, "expires_at")?,
            visibility: row.try_get("visibility")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── StoryView ─────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for StoryView {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(StoryView {
            story_id: uuid(row, "story_id")?,
            viewer_id: uuid(row, "viewer_id")?,
            viewed_at: dt(row, "viewed_at")?,
        })
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for Drawing {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(Drawing {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            author_id: uuid(row, "author_id")?,
            drawing_data: json(row, "drawing_data")?,
            width: row.try_get("width").unwrap_or(800),
            height: row.try_get("height").unwrap_or(600),
            preview_key: row
                .try_get::<Option<String>, _>("preview_key")
                .unwrap_or(None),
            message_id: opt_uuid(row, "message_id")?,
            is_whiteboard: row.try_get("is_whiteboard").unwrap_or(false),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── VoiceSettings ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for VoiceSettings {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(VoiceSettings {
            user_id: uuid(row, "user_id")?,
            spatial_audio: row.try_get("spatial_audio").unwrap_or(false),
            noise_gate_enabled: row.try_get("noise_gate_enabled").unwrap_or(true),
            noise_gate_threshold: row
                .try_get::<f32, _>("noise_gate_threshold")
                .or_else(|_| {
                    row.try_get::<f64, _>("noise_gate_threshold")
                        .map(|v| v as f32)
                })
                .unwrap_or(-50.0),
            echo_cancel_level: row
                .try_get("echo_cancel_level")
                .unwrap_or_else(|_| "moderate".to_string()),
            auto_gain: row.try_get("auto_gain").unwrap_or(true),
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── VoiceMusicQueueItem ───────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for VoiceMusicQueueItem {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(VoiceMusicQueueItem {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            added_by: uuid(row, "added_by")?,
            title: row.try_get("title")?,
            source_url: row.try_get("source_url")?,
            duration_ms: row.try_get("duration_ms")?,
            position: row.try_get("position")?,
            status: row.try_get("status")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── MediaGalleryFilter ────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for MediaGalleryFilter {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(MediaGalleryFilter {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            server_id: opt_uuid(row, "server_id")?,
            channel_id: opt_uuid(row, "channel_id")?,
            name: row.try_get("name")?,
            media_types: str_vec(row, "media_types")?,
            date_from: opt_dt(row, "date_from")?,
            date_to: opt_dt(row, "date_to")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

//  Phase 18 — Accessibility & Inclusivity

// ── UserAccessibilitySettings ─────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for UserAccessibilitySettings {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(UserAccessibilitySettings {
            user_id: uuid(row, "user_id")?,
            screen_reader_mode: row.try_get("screen_reader_mode").unwrap_or(false),
            announce_messages: row.try_get("announce_messages").unwrap_or(true),
            announce_reactions: row.try_get("announce_reactions").unwrap_or(false),
            announce_typing: row.try_get("announce_typing").unwrap_or(false),
            keyboard_shortcuts: row.try_get("keyboard_shortcuts").unwrap_or(true),
            high_contrast_mode: row.try_get("high_contrast_mode").unwrap_or(false),
            reduced_motion: row.try_get("reduced_motion").unwrap_or(false),
            font_family: row
                .try_get("font_family")
                .unwrap_or_else(|_| "system".to_string()),
            custom_font_name: row
                .try_get::<Option<String>, _>("custom_font_name")
                .unwrap_or(None),
            color_blind_mode: row
                .try_get("color_blind_mode")
                .unwrap_or_else(|_| "none".to_string()),
            preferred_language: row
                .try_get("preferred_language")
                .unwrap_or_else(|_| "en".to_string()),
            auto_translate: row.try_get("auto_translate").unwrap_or(false),
            rtl_override: row.try_get("rtl_override").unwrap_or(false),
            captions_enabled: row.try_get("captions_enabled").unwrap_or(false),
            caption_font_size: row
                .try_get("caption_font_size")
                .unwrap_or_else(|_| "md".to_string()),
            caption_position: row
                .try_get("caption_position")
                .unwrap_or_else(|_| "bottom".to_string()),
            tts_enabled: row.try_get("tts_enabled").unwrap_or(false),
            tts_rate: row
                .try_get::<f32, _>("tts_rate")
                .or_else(|_| row.try_get::<f64, _>("tts_rate").map(|v| v as f32))
                .unwrap_or(1.0),
            tts_voice: row
                .try_get("tts_voice")
                .unwrap_or_else(|_| "default".to_string()),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── VoiceCaption ──────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for VoiceCaption {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(VoiceCaption {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            speaker_id: uuid(row, "speaker_id")?,
            text: row.try_get("text")?,
            language: row.try_get("language").unwrap_or_else(|_| "en".to_string()),
            is_final: row.try_get("is_final").unwrap_or(false),
            started_at: dt(row, "started_at")?,
            ended_at: opt_dt(row, "ended_at")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── MessageTtsRequest ─────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for MessageTtsRequest {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(MessageTtsRequest {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            message_id: uuid(row, "message_id")?,
            channel_id: uuid(row, "channel_id")?,
            status: row.try_get("status")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── MessageTranslation ────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for MessageTranslation {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(MessageTranslation {
            id: uuid(row, "id")?,
            message_id: uuid(row, "message_id")?,
            source_language: row.try_get("source_language")?,
            target_language: row.try_get("target_language")?,
            translated_text: row.try_get("translated_text")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// v1.8  Ecosystem & Onboarding
// ═══════════════════════════════════════════════════════════════════════════════

// ── ImportJob ─────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ImportJob {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ImportJob {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            user_id: uuid(row, "user_id")?,
            source_platform: row.try_get("source_platform")?,
            status: row.try_get("status")?,
            total_items: row.try_get("total_items")?,
            imported_items: row.try_get("imported_items")?,
            error_log: row.try_get("error_log").unwrap_or(None),
            metadata: json(row, "metadata")?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── BulkInvitation ────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for BulkInvitation {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(BulkInvitation {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            inviter_id: uuid(row, "inviter_id")?,
            emails: json(row, "emails")?,
            status: row.try_get("status")?,
            sent_count: row.try_get("sent_count")?,
            total_count: row.try_get("total_count")?,
            invite_code: row.try_get("invite_code").unwrap_or(None),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── ServerTemplate ────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ServerTemplate {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ServerTemplate {
            id: uuid(row, "id")?,
            name: row.try_get("name")?,
            description: row.try_get("description").unwrap_or(None),
            category: row.try_get("category")?,
            icon_url: row.try_get("icon_url").unwrap_or(None),
            channels: json(row, "channels")?,
            roles: json(row, "roles")?,
            settings: json(row, "settings")?,
            is_builtin: row.try_get("is_builtin").unwrap_or(false),
            creator_id: opt_uuid(row, "creator_id")?,
            usage_count: row.try_get("usage_count").unwrap_or(0),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── OnboardingProgress ────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for OnboardingProgress {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(OnboardingProgress {
            user_id: uuid(row, "user_id")?,
            completed_steps: json(row, "completed_steps")?,
            dismissed: row.try_get("dismissed").unwrap_or(false),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── ServerAnalyticsSnapshot ───────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ServerAnalyticsSnapshot {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ServerAnalyticsSnapshot {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            period_date: row.try_get("period_date")?,
            messages_count: row.try_get("messages_count")?,
            active_members: row.try_get("active_members")?,
            new_members: row.try_get("new_members")?,
            left_members: row.try_get("left_members")?,
            voice_minutes: row.try_get("voice_minutes")?,
            reports_resolved: row.try_get("reports_resolved")?,
            bans_issued: row.try_get("bans_issued")?,
            filters_triggered: row.try_get("filters_triggered")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── MarketplacePlugin ─────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for MarketplacePlugin {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(MarketplacePlugin {
            id: uuid(row, "id")?,
            name: row.try_get("name")?,
            slug: row.try_get("slug")?,
            description: row.try_get("description").unwrap_or(None),
            author_id: opt_uuid(row, "author_id")?,
            version: row.try_get("version")?,
            manifest_url: row.try_get("manifest_url")?,
            icon_url: row.try_get("icon_url").unwrap_or(None),
            source_url: row.try_get("source_url").unwrap_or(None),
            signature: row.try_get("signature").unwrap_or(None),
            signing_key_id: row.try_get("signing_key_id").unwrap_or(None),
            category: row.try_get("category")?,
            tags: json(row, "tags")?,
            downloads: row
                .try_get::<i64, _>("downloads")
                .or_else(|_| row.try_get::<i32, _>("downloads").map(i64::from))?,
            avg_rating: row
                .try_get::<f32, _>("avg_rating")
                .or_else(|_| row.try_get::<f64, _>("avg_rating").map(|v| v as f32))?,
            rating_count: row.try_get("rating_count")?,
            is_verified: row.try_get("is_verified").unwrap_or(false),
            is_published: row.try_get("is_published").unwrap_or(true),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── PluginReview ──────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for PluginReview {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(PluginReview {
            id: uuid(row, "id")?,
            plugin_id: uuid(row, "plugin_id")?,
            user_id: uuid(row, "user_id")?,
            rating: row.try_get("rating")?,
            title: row.try_get("title").unwrap_or(None),
            body: row.try_get("body").unwrap_or(None),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── PluginInstall ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for PluginInstall {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(PluginInstall {
            id: uuid(row, "id")?,
            plugin_id: uuid(row, "plugin_id")?,
            server_id: uuid(row, "server_id")?,
            installed_by: uuid(row, "installed_by")?,
            version: row.try_get("version")?,
            is_enabled: row.try_get("is_enabled").unwrap_or(true),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── CreatorVetting ───────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for CreatorVetting {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(CreatorVetting {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            identity_level: parse_enum(row, "identity_level", |s| match s {
                "unverified" => Some(IdentityLevel::Unverified),
                "email_verified" => Some(IdentityLevel::EmailVerified),
                "domain_verified" => Some(IdentityLevel::DomainVerified),
                "signature_verified" => Some(IdentityLevel::SignatureVerified),
                _ => None,
            })?,
            domain: row.try_get("domain").unwrap_or(None),
            domain_verified: row.try_get("domain_verified").unwrap_or(false),
            signing_key_fingerprint: row.try_get("signing_key_fingerprint").unwrap_or(None),
            signing_key_type: row.try_get("signing_key_type").unwrap_or(None),
            rights_attestation: row.try_get("rights_attestation").unwrap_or(None),
            two_factor_enabled: row.try_get("two_factor_enabled").unwrap_or(false),
            ip_whitelist: row
                .try_get::<Option<String>, _>("ip_whitelist")
                .ok()
                .flatten()
                .and_then(|s| serde_json::from_str(&s).ok()),
            status: row.try_get("status")?,
            approved_by: opt_uuid(row, "approved_by")?,
            approved_at: opt_dt(row, "approved_at")?,
            rejection_reason: row.try_get("rejection_reason").unwrap_or(None),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── MarketplaceMonetization ──────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for MarketplaceMonetization {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(MarketplaceMonetization {
            id: uuid(row, "id")?,
            plugin_id: uuid(row, "plugin_id")?,
            creator_id: uuid(row, "creator_id")?,
            is_monetized: row.try_get("is_monetized").unwrap_or(false),
            price_cents: row.try_get("price_cents").unwrap_or(None),
            currency: row.try_get("currency")?,
            payment_link: row.try_get("payment_link").unwrap_or(None),
            purchase_count: row
                .try_get::<i64, _>("purchase_count")
                .or_else(|_| row.try_get::<i32, _>("purchase_count").map(i64::from))?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// v1.9 Scalability & Performance Hardening
// ═══════════════════════════════════════════════════════════════════════════════

// ── ScalingConfig ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ScalingConfig {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ScalingConfig {
            id: uuid(row, "id")?,
            instance_id: row.try_get("instance_id")?,
            region: row.try_get("region")?,
            shard_strategy: row.try_get("shard_strategy")?,
            redis_mode: row.try_get("redis_mode")?,
            gateway_weight: row.try_get("gateway_weight")?,
            max_connections: row.try_get("max_connections")?,
            metadata: json(row, "metadata")?,
            is_active: row.try_get("is_active").unwrap_or(true),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── SfuNode ───────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for SfuNode {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(SfuNode {
            id: uuid(row, "id")?,
            instance_id: row.try_get("instance_id")?,
            region: row.try_get("region")?,
            hostname: row.try_get("hostname")?,
            port: row.try_get("port")?,
            capacity: row.try_get("capacity")?,
            current_load: row.try_get("current_load")?,
            status: row.try_get("status")?,
            metadata: json(row, "metadata")?,
            last_heartbeat: dt(row, "last_heartbeat")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── FederationEventBatch ──────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for FederationEventBatch {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(FederationEventBatch {
            id: uuid(row, "id")?,
            target_instance: row.try_get("target_instance")?,
            events: json(row, "events")?,
            event_count: row.try_get("event_count")?,
            status: row.try_get("status")?,
            retry_count: row.try_get("retry_count")?,
            created_at: dt(row, "created_at")?,
            sent_at: opt_dt(row, "sent_at")?,
        })
    }
}

// ── FederationRoute ───────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for FederationRoute {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(FederationRoute {
            id: uuid(row, "id")?,
            source_instance: row.try_get("source_instance")?,
            target_instance: row.try_get("target_instance")?,
            latency_ms: row.try_get("latency_ms")?,
            is_websocket: row.try_get("is_websocket").unwrap_or(false),
            priority: row.try_get("priority")?,
            status: row.try_get("status")?,
            last_probed: dt(row, "last_probed")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── FederationDedupEntry ──────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for FederationDedupEntry {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(FederationDedupEntry {
            event_id: row.try_get("event_id")?,
            source_instance: row.try_get("source_instance")?,
            received_at: dt(row, "received_at")?,
        })
    }
}

// ── VoiceQualityLog ───────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for VoiceQualityLog {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(VoiceQualityLog {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            user_id: uuid(row, "user_id")?,
            sfu_node_id: opt_uuid(row, "sfu_node_id")?,
            bitrate: row.try_get("bitrate")?,
            packet_loss: row
                .try_get::<f64, _>("packet_loss")
                .map(|v| v as f32)
                .unwrap_or(0.0),
            jitter_ms: row
                .try_get::<f64, _>("jitter_ms")
                .map(|v| v as f32)
                .unwrap_or(0.0),
            latency_ms: row.try_get("latency_ms")?,
            fec_enabled: row.try_get("fec_enabled").unwrap_or(false),
            quality_score: row
                .try_get::<f64, _>("quality_score")
                .map(|v| v as f32)
                .unwrap_or(1.0),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── MemberPruneRule ───────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for MemberPruneRule {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(MemberPruneRule {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            inactivity_days: row.try_get("inactivity_days")?,
            grace_period_days: row.try_get("grace_period_days")?,
            exclude_roles: json(row, "exclude_roles")?,
            notify_before: row.try_get("notify_before").unwrap_or(true),
            is_enabled: row.try_get("is_enabled").unwrap_or(false),
            last_run_at: opt_dt(row, "last_run_at")?,
            pruned_count: row.try_get("pruned_count")?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── SlowModeOverride ──────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for SlowModeOverride {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(SlowModeOverride {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            role_id: opt_uuid(row, "role_id")?,
            cooldown_secs: row.try_get("cooldown_secs")?,
            escalation_mult: row
                .try_get::<f64, _>("escalation_mult")
                .map(|v| v as f32)
                .unwrap_or(1.0),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── ScalingMetric ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ScalingMetric {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ScalingMetric {
            id: uuid(row, "id")?,
            instance_id: row.try_get("instance_id")?,
            metric_name: row.try_get("metric_name")?,
            metric_value: row
                .try_get::<f64, _>("metric_value")
                .map(|v| v as f32)
                .unwrap_or(0.0),
            unit: row.try_get("unit")?,
            tags: json(row, "tags")?,
            recorded_at: dt(row, "recorded_at")?,
        })
    }
}

// ── UpgradeRecord ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for UpgradeRecord {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(UpgradeRecord {
            id: uuid(row, "id")?,
            from_version: row.try_get("from_version")?,
            to_version: row.try_get("to_version")?,
            status: row.try_get("status")?,
            started_at: dt(row, "started_at")?,
            completed_at: opt_dt(row, "completed_at")?,
            notes: row.try_get("notes").ok(),
            metadata: json(row, "metadata")?,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// v2.0 AI & Intelligence Layer
// ═══════════════════════════════════════════════════════════════════════════════

// ── SearchEmbedding ───────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for SearchEmbedding {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(SearchEmbedding {
            id: uuid(row, "id")?,
            message_id: uuid(row, "message_id")?,
            channel_id: uuid(row, "channel_id")?,
            embedding: row.try_get("embedding").ok(),
            model_name: row.try_get("model_name")?,
            model_version: row.try_get("model_version")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── SearchQuery ───────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for SearchQuery {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(SearchQuery {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            raw_query: row.try_get("raw_query")?,
            parsed_filters: json(row, "parsed_filters")?,
            result_count: row.try_get("result_count")?,
            latency_ms: row.try_get("latency_ms")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── AiSuggestion ──────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for AiSuggestion {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(AiSuggestion {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            channel_id: uuid(row, "channel_id")?,
            suggestion_type: row.try_get("suggestion_type")?,
            content: row.try_get("content")?,
            context_ids: json(row, "context_ids")?,
            model_name: row.try_get("model_name")?,
            accepted: row.try_get("accepted").ok(),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── ThreadSummary ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ThreadSummary {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ThreadSummary {
            id: uuid(row, "id")?,
            thread_id: uuid(row, "thread_id")?,
            channel_id: uuid(row, "channel_id")?,
            summary: row.try_get("summary")?,
            message_count: row.try_get("message_count")?,
            model_name: row.try_get("model_name")?,
            model_version: row.try_get("model_version")?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── ToxicityScore ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ToxicityScore {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ToxicityScore {
            id: uuid(row, "id")?,
            message_id: uuid(row, "message_id")?,
            server_id: uuid(row, "server_id")?,
            score: row
                .try_get::<f64, _>("score")
                .map(|v| v as f32)
                .unwrap_or(0.0),
            categories: json(row, "categories")?,
            model_name: row.try_get("model_name")?,
            flagged: row.try_get("flagged").unwrap_or(false),
            reviewed: row.try_get("reviewed").unwrap_or(false),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── RaidDetection ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for RaidDetection {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(RaidDetection {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            detection_type: row.try_get("detection_type")?,
            severity: row.try_get("severity")?,
            details: json(row, "details")?,
            auto_actions: json(row, "auto_actions")?,
            resolved: row.try_get("resolved").unwrap_or(false),
            detected_at: dt(row, "detected_at")?,
            resolved_at: opt_dt(row, "resolved_at")?,
        })
    }
}

// ── VoiceTranscript ───────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for VoiceTranscript {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(VoiceTranscript {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            session_id: opt_uuid(row, "session_id")?,
            speaker_id: opt_uuid(row, "speaker_id")?,
            segment_start: row
                .try_get::<f64, _>("segment_start")
                .map(|v| v as f32)
                .unwrap_or(0.0),
            segment_end: row
                .try_get::<f64, _>("segment_end")
                .map(|v| v as f32)
                .unwrap_or(0.0),
            text: row.try_get("text")?,
            language: row.try_get("language")?,
            confidence: row
                .try_get::<f64, _>("confidence")
                .map(|v| v as f32)
                .unwrap_or(0.0),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── VoiceCommand ──────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for VoiceCommand {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(VoiceCommand {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            channel_id: uuid(row, "channel_id")?,
            command_text: row.try_get("command_text")?,
            action: row.try_get("action")?,
            confidence: row
                .try_get::<f64, _>("confidence")
                .map(|v| v as f32)
                .unwrap_or(0.0),
            executed: row.try_get("executed").unwrap_or(false),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── AiConsent ─────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for AiConsent {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(AiConsent {
            user_id: uuid(row, "user_id")?,
            server_id: uuid(row, "server_id")?,
            feature: row.try_get("feature")?,
            enabled: row.try_get("enabled").unwrap_or(false),
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── AiAuditEntry ──────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for AiAuditEntry {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(AiAuditEntry {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            feature: row.try_get("feature")?,
            action: row.try_get("action")?,
            actor_id: opt_uuid(row, "actor_id")?,
            details: json(row, "details")?,
            model_name: row.try_get("model_name").ok(),
            model_version: row.try_get("model_version").ok(),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// v2.1 Voice & Real-Time Collaboration
// ═══════════════════════════════════════════════════════════════════════════════

// ── VideoLayout ───────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for VideoLayout {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(VideoLayout {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            user_id: uuid(row, "user_id")?,
            layout_type: row.try_get("layout_type")?,
            pinned_users: json(row, "pinned_users")?,
            custom_positions: json(row, "custom_positions")?,
            pip_enabled: row.try_get("pip_enabled").unwrap_or(false),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── VirtualBackground ─────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for VirtualBackground {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(VirtualBackground {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            name: row.try_get("name")?,
            bg_type: row.try_get("bg_type")?,
            image_url: row.try_get("image_url").ok(),
            is_default: row.try_get("is_default").unwrap_or(false),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── LiveStream ────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for LiveStream {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(LiveStream {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            streamer_id: uuid(row, "streamer_id")?,
            title: row.try_get("title")?,
            status: row.try_get("status")?,
            viewer_count: row.try_get("viewer_count")?,
            max_viewers: row.try_get("max_viewers")?,
            is_e2ee: row.try_get("is_e2ee").unwrap_or(false),
            recording_url: row.try_get("recording_url").ok(),
            hls_url: row.try_get("hls_url").ok(),
            started_at: dt(row, "started_at")?,
            ended_at: opt_dt(row, "ended_at")?,
        })
    }
}

// ── StreamViewer ──────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for StreamViewer {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(StreamViewer {
            stream_id: uuid(row, "stream_id")?,
            user_id: uuid(row, "user_id")?,
            joined_at: dt(row, "joined_at")?,
            left_at: opt_dt(row, "left_at")?,
        })
    }
}

// ── BreakoutRoom ──────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for BreakoutRoom {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(BreakoutRoom {
            id: uuid(row, "id")?,
            parent_channel: uuid(row, "parent_channel")?,
            name: row.try_get("name")?,
            capacity: row.try_get("capacity")?,
            status: row.try_get("status")?,
            created_by: uuid(row, "created_by")?,
            created_at: dt(row, "created_at")?,
            closed_at: opt_dt(row, "closed_at")?,
        })
    }
}

// ── CollabSession ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for CollabSession {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(CollabSession {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            session_type: row.try_get("session_type")?,
            document_id: opt_uuid(row, "document_id")?,
            participants: json(row, "participants")?,
            is_active: row.try_get("is_active").unwrap_or(true),
            created_at: dt(row, "created_at")?,
            ended_at: opt_dt(row, "ended_at")?,
        })
    }
}

// ── SpatialAudioConfig ────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for SpatialAudioConfig {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(SpatialAudioConfig {
            id: uuid(row, "id")?,
            channel_id: uuid(row, "channel_id")?,
            preset: row.try_get("preset")?,
            room_width: row
                .try_get::<f64, _>("room_width")
                .map(|v| v as f32)
                .unwrap_or(10.0),
            room_depth: row
                .try_get::<f64, _>("room_depth")
                .map(|v| v as f32)
                .unwrap_or(10.0),
            room_height: row
                .try_get::<f64, _>("room_height")
                .map(|v| v as f32)
                .unwrap_or(3.0),
            positions: json(row, "positions")?,
            hrtf_enabled: row.try_get("hrtf_enabled").unwrap_or(true),
            ambisonics_order: row.try_get("ambisonics_order")?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── VoicePreset ───────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for VoicePreset {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(VoicePreset {
            id: uuid(row, "id")?,
            name: row.try_get("name")?,
            description: row.try_get("description").ok(),
            target_latency_ms: row.try_get("target_latency_ms")?,
            jitter_buffer_ms: row.try_get("jitter_buffer_ms")?,
            fec_level: row.try_get("fec_level")?,
            dtx_enabled: row.try_get("dtx_enabled").unwrap_or(true),
            normalization: row.try_get("normalization").unwrap_or(true),
            is_builtin: row.try_get("is_builtin").unwrap_or(true),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// v2.2 User Growth & Retention
// ═══════════════════════════════════════════════════════════════════════════════

// ── ServerRecommendation ──────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ServerRecommendation {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ServerRecommendation {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            server_id: uuid(row, "server_id")?,
            score: row
                .try_get::<f64, _>("score")
                .map(|v| v as f32)
                .unwrap_or(0.0),
            reason: row.try_get("reason")?,
            dismissed: row.try_get("dismissed").unwrap_or(false),
            joined: row.try_get("joined").unwrap_or(false),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── OnboardingFlow ────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for OnboardingFlow {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(OnboardingFlow {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            steps: json(row, "steps")?,
            adaptive: row.try_get("adaptive").unwrap_or(true),
            skip_completed: row.try_get("skip_completed").unwrap_or(true),
            is_active: row.try_get("is_active").unwrap_or(true),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── DeviceSession ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for DeviceSession {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(DeviceSession {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            device_id: row.try_get("device_id")?,
            device_type: row.try_get("device_type")?,
            last_channel_id: opt_uuid(row, "last_channel_id")?,
            scroll_position: {
                let raw: Option<String> = row.try_get("scroll_position").ok();
                raw.and_then(|s| serde_json::from_str(&s).ok())
            },
            is_active: row.try_get("is_active").unwrap_or(true),
            last_seen_at: dt(row, "last_seen_at")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── ClipboardSync ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ClipboardSync {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ClipboardSync {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            source_device: row.try_get("source_device")?,
            content_type: row.try_get("content_type")?,
            encrypted_data: row.try_get("encrypted_data")?,
            expires_at: dt(row, "expires_at")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── UserXp ────────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for UserXp {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(UserXp {
            user_id: uuid(row, "user_id")?,
            server_id: uuid(row, "server_id")?,
            xp: row.try_get::<i64, _>("xp").unwrap_or(0),
            level: row.try_get("level")?,
            last_xp_at: dt(row, "last_xp_at")?,
        })
    }
}

// ── GamificationConfig ────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for GamificationConfig {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(GamificationConfig {
            server_id: uuid(row, "server_id")?,
            enabled: row.try_get("enabled").unwrap_or(false),
            xp_per_message: row.try_get("xp_per_message")?,
            xp_per_reaction: row.try_get("xp_per_reaction")?,
            xp_per_voice_min: row.try_get("xp_per_voice_min")?,
            level_formula: row.try_get("level_formula")?,
            streak_enabled: row.try_get("streak_enabled").unwrap_or(true),
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── Achievement ───────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for Achievement {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(Achievement {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            name: row.try_get("name")?,
            description: row.try_get("description").ok(),
            icon_url: row.try_get("icon_url").ok(),
            criteria: json(row, "criteria")?,
            reward_xp: row.try_get("reward_xp")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── UserAchievement ───────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for UserAchievement {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(UserAchievement {
            user_id: uuid(row, "user_id")?,
            achievement_id: uuid(row, "achievement_id")?,
            earned_at: dt(row, "earned_at")?,
        })
    }
}

// ── ActivityStreak ────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ActivityStreak {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ActivityStreak {
            user_id: uuid(row, "user_id")?,
            server_id: uuid(row, "server_id")?,
            current_streak: row.try_get("current_streak")?,
            longest_streak: row.try_get("longest_streak")?,
            last_active_date: row.try_get("last_active_date")?,
        })
    }
}

// ── SyncCursor ────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for SyncCursor {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(SyncCursor {
            user_id: uuid(row, "user_id")?,
            device_id: row.try_get("device_id")?,
            channel_id: uuid(row, "channel_id")?,
            last_message_id: opt_uuid(row, "last_message_id")?,
            last_synced_at: dt(row, "last_synced_at")?,
        })
    }
}

// ── OfflineQueueItem ──────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for OfflineQueueItem {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(OfflineQueueItem {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            device_id: row.try_get("device_id")?,
            action_type: row.try_get("action_type")?,
            payload: json(row, "payload")?,
            status: row.try_get("status")?,
            conflict_data: {
                let raw: Option<String> = row.try_get("conflict_data").ok();
                raw.and_then(|s| serde_json::from_str(&s).ok())
            },
            created_at: dt(row, "created_at")?,
            synced_at: opt_dt(row, "synced_at")?,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// v2.x Sustainability & Extensibility
// ═══════════════════════════════════════════════════════════════════════════════

// ── ProtocolVersion ───────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ProtocolVersion {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ProtocolVersion {
            id: uuid(row, "id")?,
            protocol: row.try_get("protocol")?,
            version: row.try_get("version")?,
            status: row.try_get("status")?,
            capabilities: json(row, "capabilities")?,
            deprecation_date: opt_dt(row, "deprecation_date")?,
            sunset_date: opt_dt(row, "sunset_date")?,
            migration_guide: row.try_get("migration_guide").ok(),
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── CapabilityNegotiation ─────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for CapabilityNegotiation {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(CapabilityNegotiation {
            id: uuid(row, "id")?,
            local_instance: row.try_get("local_instance")?,
            remote_instance: row.try_get("remote_instance")?,
            local_caps: json(row, "local_caps")?,
            remote_caps: json(row, "remote_caps")?,
            agreed_caps: json(row, "agreed_caps")?,
            protocol_version: row.try_get("protocol_version")?,
            negotiated_at: dt(row, "negotiated_at")?,
        })
    }
}

// ── GovernancePoll ────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for GovernancePoll {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(GovernancePoll {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            title: row.try_get("title")?,
            description: row.try_get("description").ok(),
            poll_type: row.try_get("poll_type")?,
            options: json(row, "options")?,
            min_participation: row
                .try_get::<f64, _>("min_participation")
                .map(|v| v as f32)
                .unwrap_or(0.0),
            allow_multiple: row.try_get("allow_multiple").unwrap_or(false),
            anonymous: row.try_get("anonymous").unwrap_or(true),
            status: row.try_get("status")?,
            created_by: uuid(row, "created_by")?,
            opens_at: dt(row, "opens_at")?,
            closes_at: opt_dt(row, "closes_at")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── PollVote ──────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for PollVote {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(PollVote {
            poll_id: uuid(row, "poll_id")?,
            user_id: uuid(row, "user_id")?,
            option_index: row.try_get("option_index")?,
            voted_at: dt(row, "voted_at")?,
        })
    }
}

// ── GovernanceProposal ────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for GovernanceProposal {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(GovernanceProposal {
            id: uuid(row, "id")?,
            server_id: uuid(row, "server_id")?,
            title: row.try_get("title")?,
            body: row.try_get("body")?,
            status: row.try_get("status")?,
            author_id: uuid(row, "author_id")?,
            discussion_channel: opt_uuid(row, "discussion_channel")?,
            poll_id: opt_uuid(row, "poll_id")?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── ContributorBadge ──────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for ContributorBadge {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(ContributorBadge {
            id: uuid(row, "id")?,
            user_id: uuid(row, "user_id")?,
            badge_type: row.try_get("badge_type")?,
            source: row.try_get("source").ok(),
            verified: row.try_get("verified").unwrap_or(false),
            metadata: json(row, "metadata")?,
            awarded_at: dt(row, "awarded_at")?,
        })
    }
}

// ── SecurityAudit ─────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for SecurityAudit {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(SecurityAudit {
            id: uuid(row, "id")?,
            audit_type: row.try_get("audit_type")?,
            status: row.try_get("status")?,
            findings: json(row, "findings")?,
            severity_summary: json(row, "severity_summary")?,
            auditor: row.try_get("auditor").ok(),
            started_at: opt_dt(row, "started_at")?,
            completed_at: opt_dt(row, "completed_at")?,
            created_at: dt(row, "created_at")?,
        })
    }
}

// ── VulnerabilityRecord ───────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for VulnerabilityRecord {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(VulnerabilityRecord {
            id: uuid(row, "id")?,
            audit_id: opt_uuid(row, "audit_id")?,
            cve_id: row.try_get("cve_id").ok(),
            package_name: row.try_get("package_name")?,
            severity: row.try_get("severity")?,
            description: row.try_get("description")?,
            remediation: row.try_get("remediation").ok(),
            status: row.try_get("status")?,
            discovered_at: dt(row, "discovered_at")?,
            resolved_at: opt_dt(row, "resolved_at")?,
        })
    }
}

// ── TutorialProgress ──────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for TutorialProgress {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(TutorialProgress {
            user_id: uuid(row, "user_id")?,
            tutorial_id: row.try_get("tutorial_id")?,
            completed_steps: json(row, "completed_steps")?,
            completed: row.try_get("completed").unwrap_or(false),
            started_at: dt(row, "started_at")?,
            completed_at: opt_dt(row, "completed_at")?,
        })
    }
}

// ── MigrationGuide ────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for MigrationGuide {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(MigrationGuide {
            id: uuid(row, "id")?,
            from_platform: row.try_get("from_platform")?,
            title: row.try_get("title")?,
            content: row.try_get("content")?,
            version: row.try_get("version")?,
            is_published: row.try_get("is_published").unwrap_or(false),
            author_id: opt_uuid(row, "author_id")?,
            created_at: dt(row, "created_at")?,
            updated_at: dt(row, "updated_at")?,
        })
    }
}

// ── PhantomIdentity ───────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, AnyRow> for PhantomIdentity {
    fn from_row(row: &'r AnyRow) -> Result<Self, sqlx::Error> {
        Ok(PhantomIdentity {
            user_id: uuid(row, "user_id")?,
            did: row.try_get("did")?,
            kem_public: row.try_get("kem_public")?,
            signing_public: row.try_get("signing_public")?,
            created_at: dt(row, "created_at")?,
        })
    }
}
