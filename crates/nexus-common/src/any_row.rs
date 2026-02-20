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
    crypto::{Device, DeviceType, DeviceVerification, E2eeChannel, E2eeSession, EncryptedMessage, OneTimePreKey, VerificationMethod},
    member::Member,
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
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Ok(d.with_timezone(&Utc));
    }
    if let Ok(d) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(d.and_utc());
    }
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
