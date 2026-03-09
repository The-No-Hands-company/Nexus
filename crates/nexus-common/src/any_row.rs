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

use chrono::{DateTime, Utc};
use sqlx::{any::AnyRow, Row};
use uuid::Uuid;

use crate::models::{
    channel::{Channel, ChannelType},
    collaboration::{
        AiPreferences, CalendarEvent, CalendarRsvp, ChannelDigest,
        ChecklistItem, FileVersion, ServerStorageQuota, Task, TaskReminder,
    },
    crypto::{Device, DeviceType, DeviceVerification, E2eeChannel, E2eeSession, EncryptedMessage, OneTimePreKey, VerificationMethod},
    member::Member,
    multimedia::{
        Drawing, MediaGalleryFilter, Story, StoryView, VideoNote, VoiceMusicQueueItem,
        VoiceNote, VoiceSettings,
    },
    relationship::{Relationship, RelationshipStatus},
    rich::{AttachmentRow, ServerEmojiRow, ThreadRow},
    role::Role,
    server::{Invite, Server},
    user::{User, UserPresence},
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
    parse_dt(&s).map_err(|e| sqlx::Error::Decode(e))
}

fn opt_dt(row: &AnyRow, col: &str) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    let s: Option<String> = row.try_get(col)?;
    s.map(|v| parse_dt(&v).map_err(|e| sqlx::Error::Decode(e)))
        .transpose()
}

fn parse_dt(
    s: &str,
) -> Result<
    DateTime<Utc>,
    Box<dyn std::error::Error + Send + Sync + 'static>,
> {
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
                format!("{}:00", iso)
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

fn parse_enum<T>(
    row: &AnyRow,
    col: &str,
    f: impl Fn(&str) -> Option<T>,
) -> Result<T, sqlx::Error> {
    let s: String = row.try_get(col)?;
    f(&s).ok_or_else(|| sqlx::Error::Decode(format!("unknown enum variant: {s}").into()))
}

fn opt_enum<T>(
    row: &AnyRow,
    col: &str,
    f: impl Fn(&str) -> Option<T>,
) -> Result<Option<T>, sqlx::Error> {
    let s: Option<String> = row.try_get(col)?;
    s.map(|v| {
        f(&v).ok_or_else(|| sqlx::Error::Decode(format!("unknown enum variant: {v}").into()))
    })
    .transpose()
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
            size: row.try_get::<i64, _>("size").or_else(|_| {
                row.try_get::<i32, _>("size").map(|v| v as i64)
            }).unwrap_or(0),
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
            max_bytes: row.try_get::<i64, _>("max_bytes").or_else(|_| {
                row.try_get::<i32, _>("max_bytes").map(|v| v as i64)
            }).unwrap_or(0),
            used_bytes: row.try_get::<i64, _>("used_bytes").or_else(|_| {
                row.try_get::<i32, _>("used_bytes").map(|v| v as i64)
            }).unwrap_or(0),
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
            digest_interval: row.try_get("digest_interval").unwrap_or_else(|_| "daily".to_string()),
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
            size: row.try_get::<i64, _>("size").or_else(|_| {
                row.try_get::<i32, _>("size").map(|v| v as i64)
            })?,
            duration_ms: row.try_get("duration_ms")?,
            waveform: row.try_get::<Option<String>, _>("waveform").unwrap_or(None),
            transcript: row.try_get::<Option<String>, _>("transcript").unwrap_or(None),
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
            size: row.try_get::<i64, _>("size").or_else(|_| {
                row.try_get::<i32, _>("size").map(|v| v as i64)
            })?,
            duration_ms: row.try_get("duration_ms")?,
            width: row.try_get::<Option<i32>, _>("width").unwrap_or(None),
            height: row.try_get::<Option<i32>, _>("height").unwrap_or(None),
            thumbnail_key: row.try_get::<Option<String>, _>("thumbnail_key").unwrap_or(None),
            transcript: row.try_get::<Option<String>, _>("transcript").unwrap_or(None),
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
            storage_key: row.try_get::<Option<String>, _>("storage_key").unwrap_or(None),
            text_content: row.try_get::<Option<String>, _>("text_content").unwrap_or(None),
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
            preview_key: row.try_get::<Option<String>, _>("preview_key").unwrap_or(None),
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
            noise_gate_threshold: row.try_get::<f32, _>("noise_gate_threshold")
                .or_else(|_| row.try_get::<f64, _>("noise_gate_threshold").map(|v| v as f32))
                .unwrap_or(-50.0),
            echo_cancel_level: row.try_get("echo_cancel_level").unwrap_or_else(|_| "moderate".to_string()),
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
