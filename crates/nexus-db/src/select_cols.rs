//! Per-table SELECT column lists with Postgres-compatible type casts.
//!
//! `AnyPool` cannot decode Postgres-native types (UUID, ENUM, TIMESTAMPTZ,
//! JSONB, UUID[]).  Every column of those types must be cast to `::text` so the
//! Any driver hands them to our manual `FromRow` helpers as plain `String`s.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::select_cols::USER_COLS;
//! let q = format!("SELECT {USER_COLS} FROM users WHERE id = $1");
//! // sqlx::query_as::<_, User>(&q).bind(id).fetch_optional(pool).await
//! ```

// ── users ─────────────────────────────────────────────────────────────────────
pub const USER_COLS: &str =
    "id::text AS id, username, display_name, email, password_hash, avatar, banner, \
     bio, status, presence::text AS presence, flags, totp_enabled, \
     created_at::text AS created_at, updated_at::text AS updated_at";

// ── servers ───────────────────────────────────────────────────────────────────
pub const SERVER_COLS: &str =
    "id::text AS id, name, description, icon, banner, owner_id::text AS owner_id, \
     region, is_public, features::text AS features, settings::text AS settings, \
     vanity_code, member_count, max_file_size, require_2fa, spam_window_secs, spam_max_messages, \
     created_at::text AS created_at, updated_at::text AS updated_at";

/// Same as `SERVER_COLS` but with `s.` table-alias prefix for JOIN queries.
pub const SERVER_COLS_S: &str =
    "s.id::text AS id, s.name, s.description, s.icon, s.banner, \
     s.owner_id::text AS owner_id, s.region, s.is_public, \
     s.features::text AS features, s.settings::text AS settings, \
     s.vanity_code, s.member_count, s.max_file_size, s.require_2fa, s.spam_window_secs, s.spam_max_messages, \
     s.created_at::text AS created_at, s.updated_at::text AS updated_at";

// ── channels ──────────────────────────────────────────────────────────────────
pub const CHANNEL_COLS: &str =
    "id::text AS id, server_id::text AS server_id, parent_id::text AS parent_id, \
     channel_type::text AS channel_type, name, topic, position, nsfw, \
     rate_limit_per_user, bitrate, user_limit, encrypted, \
     permission_overwrites::text AS permission_overwrites, \
     last_message_id::text AS last_message_id, auto_archive_duration, \
     archived, locked, disappear_after_seconds, is_stream, \
     created_at::text AS created_at, updated_at::text AS updated_at";

/// Same as `CHANNEL_COLS` but with `c.` table-alias prefix for JOIN queries.
pub const CHANNEL_COLS_C: &str =
    "c.id::text AS id, c.server_id::text AS server_id, c.parent_id::text AS parent_id, \
     c.channel_type::text AS channel_type, c.name, c.topic, c.position, c.nsfw, \
     c.rate_limit_per_user, c.bitrate, c.user_limit, c.encrypted, \
     c.permission_overwrites::text AS permission_overwrites, \
     c.last_message_id::text AS last_message_id, c.auto_archive_duration, \
     c.archived, c.locked, c.disappear_after_seconds, c.is_stream, \
     c.created_at::text AS created_at, c.updated_at::text AS updated_at";

// ── members ───────────────────────────────────────────────────────────────────
/// UUID[] `roles` cast via `array_to_json` → JSON string, readable by the
/// `uuid_vec` helper.
pub const MEMBER_COLS: &str =
    "user_id::text AS user_id, server_id::text AS server_id, nickname, avatar, \
     COALESCE(array_to_json(roles), '[]'::json)::text AS roles, \
     muted, deafened, joined_at::text AS joined_at, \
     communication_disabled_until::text AS communication_disabled_until";

// ── roles ─────────────────────────────────────────────────────────────────────
pub const ROLE_COLS: &str =
    "id::text AS id, server_id::text AS server_id, name, color, hoist, icon, \
     position, permissions, mentionable, is_default, \
     created_at::text AS created_at, updated_at::text AS updated_at";

// ── invites ───────────────────────────────────────────────────────────────────
pub const INVITE_COLS: &str =
    "code, server_id::text AS server_id, channel_id::text AS channel_id, \
     inviter_id::text AS inviter_id, max_uses, uses, \
     expires_at::text AS expires_at, created_at::text AS created_at";

// ── messages ──────────────────────────────────────────────────────────────────
/// UUID[] columns (mentions, mention_roles) are cast via array_to_json → ::text
/// so the `get_uuid_vec` helper can parse them.
pub const MESSAGE_COLS: &str =
    "id::text AS id, channel_id::text AS channel_id, author_id::text AS author_id, \
     content, message_type, edited, edited_at::text AS edited_at, pinned, \
     embeds::text AS embeds, attachments::text AS attachments, \
     COALESCE(array_to_json(mentions), '[]'::json)::text AS mentions, \
     COALESCE(array_to_json(mention_roles), '[]'::json)::text AS mention_roles, \
     mention_everyone, \
     reference_message_id::text AS reference_message_id, \
     reference_channel_id::text AS reference_channel_id, \
     thread_id::text AS thread_id, \
     flags, created_at::text AS created_at, updated_at::text AS updated_at";

/// Same as `MESSAGE_COLS` but with `m.` table-alias prefix for JOIN queries.
pub const MESSAGE_COLS_M: &str =
    "m.id::text AS id, m.channel_id::text AS channel_id, m.author_id::text AS author_id, \
     m.content, m.message_type, m.edited, m.edited_at::text AS edited_at, m.pinned, \
     m.embeds::text AS embeds, m.attachments::text AS attachments, \
     COALESCE(array_to_json(m.mentions), '[]'::json)::text AS mentions, \
     COALESCE(array_to_json(m.mention_roles), '[]'::json)::text AS mention_roles, \
     m.mention_everyone, \
     m.reference_message_id::text AS reference_message_id, \
     m.reference_channel_id::text AS reference_channel_id, \
     m.thread_id::text AS thread_id, \
     m.flags, m.created_at::text AS created_at, m.updated_at::text AS updated_at";
// ── user_relationships ────────────────────────────────────────────────────────
pub const RELATIONSHIP_COLS: &str =
    "id::text AS id, requester_id::text AS requester_id, addressee_id::text AS addressee_id, \
     status::text AS status, created_at::text AS created_at, updated_at::text AS updated_at";

// ── server_emoji ──────────────────────────────────────────────────────────────
pub const EMOJI_COLS: &str =
    "id::text AS id, server_id::text AS server_id, creator_id::text AS creator_id, \
     name, storage_key, url, animated, managed, available, \
     created_at::text AS created_at";

// ── attachments ───────────────────────────────────────────────────────────────
pub const ATTACHMENT_COLS: &str =
    "id::text AS id, uploader_id::text AS uploader_id, server_id::text AS server_id, \
     channel_id::text AS channel_id, message_id::text AS message_id, \
     filename, content_type, size, storage_key, url, \
     width, height, duration_secs, spoiler, blurhash, sha256, status, \
     created_at::text AS created_at, updated_at::text AS updated_at";

/// Same as `ATTACHMENT_COLS` but with `a.` table-alias prefix for JOIN queries.
pub const ATTACHMENT_COLS_A: &str =
    "a.id::text AS id, a.uploader_id::text AS uploader_id, a.server_id::text AS server_id, \
     a.channel_id::text AS channel_id, a.message_id::text AS message_id, \
     a.filename, a.content_type, a.size, a.storage_key, a.url, \
     a.width, a.height, a.duration_secs, a.spoiler, a.blurhash, a.sha256, a.status, \
     a.created_at::text AS created_at, a.updated_at::text AS updated_at";

// ── threads ───────────────────────────────────────────────────────────────────
/// Columns for the `threads` table (no prefix).  Used in RETURNING clauses
/// where `parent_channel_id` is appended as a separate expression.
pub const THREAD_COLS: &str =
    "channel_id::text AS channel_id, parent_message_id::text AS parent_message_id, \
     owner_id::text AS owner_id, title, message_count, member_count, \
     auto_archive_minutes, archived, archived_at::text AS archived_at, locked, \
     COALESCE(array_to_json(tags), '[]'::json)::text AS tags, \
     created_at::text AS created_at, updated_at::text AS updated_at";

/// Same as `THREAD_COLS` but with `t.` table-alias prefix for JOIN queries.
pub const THREAD_COLS_T: &str =
    "t.channel_id::text AS channel_id, t.parent_message_id::text AS parent_message_id, \
     t.owner_id::text AS owner_id, t.title, t.message_count, t.member_count, \
     t.auto_archive_minutes, t.archived, t.archived_at::text AS archived_at, t.locked, \
     COALESCE(array_to_json(t.tags), '[]'::json)::text AS tags, \
     t.created_at::text AS created_at, t.updated_at::text AS updated_at";