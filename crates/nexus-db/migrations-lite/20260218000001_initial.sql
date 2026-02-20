-- Nexus SQLite Schema (lite mode)
-- Single consolidated migration — covers all features.
-- Types: TEXT for UUIDs/timestamps/JSON, INTEGER for booleans/bigints.
-- No ENUMs: TEXT columns with CHECK constraints where meaningful.
-- No PL/pgSQL triggers; no tsvector; no ARRAY types.

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- ============================================================
-- Users
-- ============================================================
CREATE TABLE IF NOT EXISTS users (
    id              TEXT PRIMARY KEY,
    username        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name    TEXT,
    email           TEXT UNIQUE COLLATE NOCASE,
    password_hash   TEXT NOT NULL,
    avatar          TEXT,
    banner          TEXT,
    bio             TEXT,
    status          TEXT,
    presence        TEXT NOT NULL DEFAULT 'offline',
    flags           INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users (username COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_users_created_at ON users (created_at);

-- ============================================================
-- Servers
-- ============================================================
CREATE TABLE IF NOT EXISTS servers (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    description     TEXT,
    icon            TEXT,
    banner          TEXT,
    owner_id        TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    region          TEXT,
    is_public       INTEGER NOT NULL DEFAULT 0,
    features        TEXT NOT NULL DEFAULT '{}',
    settings        TEXT NOT NULL DEFAULT '{}',
    vanity_code     TEXT UNIQUE,
    member_count    INTEGER NOT NULL DEFAULT 0,
    max_file_size   INTEGER,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_servers_owner    ON servers (owner_id);
CREATE INDEX IF NOT EXISTS idx_servers_vanity   ON servers (vanity_code);

-- ============================================================
-- Channels
-- ============================================================
CREATE TABLE IF NOT EXISTS channels (
    id                      TEXT PRIMARY KEY,
    server_id               TEXT REFERENCES servers(id) ON DELETE CASCADE,
    parent_id               TEXT REFERENCES channels(id) ON DELETE SET NULL,
    channel_type            TEXT NOT NULL,
    name                    TEXT,
    topic                   TEXT,
    position                INTEGER NOT NULL DEFAULT 0,
    nsfw                    INTEGER NOT NULL DEFAULT 0,
    rate_limit_per_user     INTEGER NOT NULL DEFAULT 0,
    bitrate                 INTEGER,
    user_limit              INTEGER,
    encrypted               INTEGER NOT NULL DEFAULT 0,
    permission_overwrites   TEXT NOT NULL DEFAULT '[]',
    last_message_id         TEXT,
    auto_archive_duration   INTEGER,
    archived                INTEGER NOT NULL DEFAULT 0,
    locked                  INTEGER NOT NULL DEFAULT 0,
    created_at              TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at              TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_channels_server  ON channels (server_id, position);
CREATE INDEX IF NOT EXISTS idx_channels_parent  ON channels (parent_id);
CREATE INDEX IF NOT EXISTS idx_channels_type    ON channels (channel_type);

-- ============================================================
-- DM Participants
-- ============================================================
CREATE TABLE IF NOT EXISTS dm_participants (
    channel_id  TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (channel_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_dm_participants_user ON dm_participants (user_id);

-- ============================================================
-- Roles
-- ============================================================
CREATE TABLE IF NOT EXISTS roles (
    id          TEXT PRIMARY KEY,
    server_id   TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    color       INTEGER,
    hoist       INTEGER NOT NULL DEFAULT 0,
    icon        TEXT,
    position    INTEGER NOT NULL DEFAULT 0,
    permissions INTEGER NOT NULL DEFAULT 0,
    mentionable INTEGER NOT NULL DEFAULT 1,
    is_default  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_roles_server ON roles (server_id, position DESC);

-- ============================================================
-- Members
-- ============================================================
CREATE TABLE IF NOT EXISTS members (
    user_id                         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id                       TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    nickname                        TEXT,
    avatar                          TEXT,
    roles                           TEXT NOT NULL DEFAULT '[]',  -- JSON array of role IDs
    muted                           INTEGER NOT NULL DEFAULT 0,
    deafened                        INTEGER NOT NULL DEFAULT 0,
    joined_at                       TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    communication_disabled_until    TEXT,
    PRIMARY KEY (user_id, server_id)
);

CREATE INDEX IF NOT EXISTS idx_members_server ON members (server_id, joined_at);
CREATE INDEX IF NOT EXISTS idx_members_user   ON members (user_id);

-- ============================================================
-- Invites
-- ============================================================
CREATE TABLE IF NOT EXISTS invites (
    code        TEXT PRIMARY KEY,
    server_id   TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    channel_id  TEXT REFERENCES channels(id) ON DELETE SET NULL,
    inviter_id  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    max_uses    INTEGER,
    uses        INTEGER NOT NULL DEFAULT 0,
    expires_at  TEXT,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_invites_server  ON invites (server_id);
CREATE INDEX IF NOT EXISTS idx_invites_expires ON invites (expires_at);

-- ============================================================
-- Refresh Tokens
-- ============================================================
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    device_info TEXT,
    ip_address  TEXT,
    expires_at  TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user    ON refresh_tokens (user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires ON refresh_tokens (expires_at);

-- ============================================================
-- Bans
-- ============================================================
CREATE TABLE IF NOT EXISTS bans (
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id   TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    reason      TEXT,
    banned_by   TEXT NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, server_id)
);

-- ============================================================
-- Emoji
-- ============================================================
CREATE TABLE IF NOT EXISTS emojis (
    id          TEXT PRIMARY KEY,
    server_id   TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    image_key   TEXT NOT NULL,
    creator_id  TEXT REFERENCES users(id) ON DELETE SET NULL,
    animated    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_emojis_server ON emojis (server_id);

-- ============================================================
-- Messages
-- ============================================================
CREATE TABLE IF NOT EXISTS messages (
    id                      TEXT PRIMARY KEY,
    channel_id              TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id               TEXT NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    content                 TEXT NOT NULL DEFAULT '',
    message_type            INTEGER NOT NULL DEFAULT 0,
    edited                  INTEGER NOT NULL DEFAULT 0,
    edited_at               TEXT,
    pinned                  INTEGER NOT NULL DEFAULT 0,
    embeds                  TEXT NOT NULL DEFAULT '[]',
    attachments             TEXT NOT NULL DEFAULT '[]',
    mentions                TEXT NOT NULL DEFAULT '[]',   -- JSON array
    mention_roles           TEXT NOT NULL DEFAULT '[]',   -- JSON array
    mention_everyone        INTEGER NOT NULL DEFAULT 0,
    reference_message_id    TEXT REFERENCES messages(id) ON DELETE SET NULL,
    reference_channel_id    TEXT REFERENCES channels(id) ON DELETE SET NULL,
    thread_id               TEXT REFERENCES channels(id) ON DELETE SET NULL,
    flags                   INTEGER NOT NULL DEFAULT 0,
    author_username         TEXT NOT NULL DEFAULT '',
    created_at              TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at              TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_messages_channel_created ON messages (channel_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_author          ON messages (author_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_pinned          ON messages (channel_id, pinned);
CREATE INDEX IF NOT EXISTS idx_messages_reference       ON messages (reference_message_id);

-- ============================================================
-- Reactions
-- ============================================================
CREATE TABLE IF NOT EXISTS reactions (
    message_id  TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji       TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (message_id, user_id, emoji)
);

CREATE INDEX IF NOT EXISTS idx_reactions_message ON reactions (message_id, emoji);
CREATE INDEX IF NOT EXISTS idx_reactions_user    ON reactions (user_id);

-- ============================================================
-- Read States
-- ============================================================
CREATE TABLE IF NOT EXISTS read_states (
    user_id              TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id           TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    last_read_message_id TEXT REFERENCES messages(id) ON DELETE SET NULL,
    mention_count        INTEGER NOT NULL DEFAULT 0,
    last_read_at         TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, channel_id)
);

CREATE INDEX IF NOT EXISTS idx_read_states_user ON read_states (user_id);

-- ============================================================
-- Attachments
-- ============================================================
CREATE TABLE IF NOT EXISTS attachments (
    id              TEXT PRIMARY KEY,
    uploader_id     TEXT NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    server_id       TEXT REFERENCES servers(id) ON DELETE SET NULL,
    channel_id      TEXT REFERENCES channels(id) ON DELETE SET NULL,
    message_id      TEXT REFERENCES messages(id) ON DELETE SET NULL,
    filename        TEXT NOT NULL,
    content_type    TEXT NOT NULL,
    size            INTEGER NOT NULL,
    storage_key     TEXT NOT NULL UNIQUE,
    url             TEXT,
    width           INTEGER,
    height          INTEGER,
    duration_secs   REAL,
    spoiler         INTEGER NOT NULL DEFAULT 0,
    blurhash        TEXT,
    sha256          TEXT,
    status          TEXT NOT NULL DEFAULT 'pending',
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_attachments_message   ON attachments (message_id);
CREATE INDEX IF NOT EXISTS idx_attachments_uploader  ON attachments (uploader_id);

-- ============================================================
-- Threads
-- ============================================================
CREATE TABLE IF NOT EXISTS threads (
    id                  TEXT PRIMARY KEY REFERENCES channels(id) ON DELETE CASCADE,
    parent_channel_id   TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    creator_id          TEXT NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    message_count       INTEGER NOT NULL DEFAULT 0,
    member_count        INTEGER NOT NULL DEFAULT 0,
    archive_timestamp   TEXT,
    created_at          TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_threads_parent  ON threads (parent_channel_id);
CREATE INDEX IF NOT EXISTS idx_threads_creator ON threads (creator_id);

-- ============================================================
-- Pinned Messages
-- ============================================================
CREATE TABLE IF NOT EXISTS pinned_messages (
    channel_id  TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    message_id  TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    pinned_by   TEXT NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    pinned_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (channel_id, message_id)
);

-- ============================================================
-- MeiliSearch sync queue (no-op in lite mode — here for schema compat)
-- ============================================================
CREATE TABLE IF NOT EXISTS search_sync_queue (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    action      TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id   TEXT NOT NULL,
    data        TEXT,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed   INTEGER NOT NULL DEFAULT 0
);

-- ============================================================
-- E2EE — Devices
-- ============================================================
CREATE TABLE IF NOT EXISTS devices (
    id                      TEXT PRIMARY KEY,
    user_id                 TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name                    TEXT NOT NULL DEFAULT '',
    identity_key            TEXT NOT NULL,
    signed_pre_key          TEXT NOT NULL,
    signed_pre_key_sig      TEXT NOT NULL,
    signed_pre_key_id       INTEGER NOT NULL DEFAULT 0,
    device_type             TEXT NOT NULL DEFAULT 'unknown',
    last_seen_at            TEXT,
    verified                INTEGER NOT NULL DEFAULT 0,
    created_at              TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at              TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (user_id, identity_key)
);

CREATE INDEX IF NOT EXISTS idx_devices_user ON devices (user_id);

-- ============================================================
-- E2EE — One-Time Pre-Keys
-- ============================================================
CREATE TABLE IF NOT EXISTS one_time_pre_keys (
    id          TEXT PRIMARY KEY,
    device_id   TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    key_id      INTEGER NOT NULL,
    public_key  TEXT NOT NULL,
    used        INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (device_id, key_id)
);

CREATE INDEX IF NOT EXISTS idx_otpk_device ON one_time_pre_keys (device_id, used);

-- ============================================================
-- E2EE — Sessions (per sender-recipient-device triple)
-- ============================================================
CREATE TABLE IF NOT EXISTS e2ee_sessions (
    id                      TEXT PRIMARY KEY,
    sender_device_id        TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    recipient_device_id     TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    session_state           TEXT NOT NULL,
    created_at              TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at              TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (sender_device_id, recipient_device_id)
);

-- ============================================================
-- Encrypted Messages (E2EE content — server stores ciphertext only)
-- ============================================================
CREATE TABLE IF NOT EXISTS encrypted_messages (
    id              TEXT PRIMARY KEY,
    channel_id      TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    sender_id       TEXT NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    ciphertext_map  TEXT NOT NULL DEFAULT '{}',   -- JSON: {device_id: ciphertext}
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_encrypted_messages_channel ON encrypted_messages (channel_id, created_at DESC);

-- ============================================================
-- Key Backup (encrypted master key backup per user)
-- ============================================================
CREATE TABLE IF NOT EXISTS key_backups (
    user_id         TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    encrypted_key   TEXT NOT NULL,
    iv              TEXT NOT NULL,
    updated_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- Bots
-- ============================================================
CREATE TABLE IF NOT EXISTS bots (
    id              TEXT PRIMARY KEY,
    owner_id        TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    description     TEXT,
    avatar          TEXT,
    token_hash      TEXT NOT NULL UNIQUE,
    permissions     INTEGER NOT NULL DEFAULT 0,
    is_public       INTEGER NOT NULL DEFAULT 0,
    is_verified     INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_bots_owner ON bots (owner_id);

-- ============================================================
-- Bot Server Memberships
-- ============================================================
CREATE TABLE IF NOT EXISTS bot_members (
    bot_id      TEXT NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    server_id   TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    added_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (bot_id, server_id)
);

-- ============================================================
-- Webhooks
-- ============================================================
CREATE TABLE IF NOT EXISTS webhooks (
    id          TEXT PRIMARY KEY,
    channel_id  TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    server_id   TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    creator_id  TEXT NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    name        TEXT NOT NULL,
    avatar      TEXT,
    token       TEXT NOT NULL UNIQUE,
    type        TEXT NOT NULL DEFAULT 'incoming',
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_webhooks_channel ON webhooks (channel_id);

-- ============================================================
-- Slash Commands
-- ============================================================
CREATE TABLE IF NOT EXISTS slash_commands (
    id              TEXT PRIMARY KEY,
    bot_id          TEXT NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    options         TEXT NOT NULL DEFAULT '[]',
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (bot_id, name)
);

-- ============================================================
-- Plugins (client-side plugin registry)
-- ============================================================
CREATE TABLE IF NOT EXISTS plugins (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    url         TEXT NOT NULL,
    version     TEXT NOT NULL DEFAULT '0.0.0',
    enabled     INTEGER NOT NULL DEFAULT 1,
    manifest    TEXT NOT NULL DEFAULT '{}',
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- Federation: Server registry
-- ============================================================
CREATE TABLE IF NOT EXISTS federated_servers (
    id                  TEXT PRIMARY KEY,
    server_name         TEXT NOT NULL UNIQUE,
    public_key          TEXT NOT NULL,
    signing_algorithm   TEXT NOT NULL DEFAULT 'ed25519',
    trusted             INTEGER NOT NULL DEFAULT 0,
    blocked             INTEGER NOT NULL DEFAULT 0,
    last_seen_at        TEXT,
    created_at          TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- Federation: Remote users
-- ============================================================
CREATE TABLE IF NOT EXISTS federated_users (
    id              TEXT PRIMARY KEY,
    mxid            TEXT NOT NULL UNIQUE,     -- @user:server.tld
    display_name    TEXT,
    avatar_url      TEXT,
    server_name     TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_federated_users_server ON federated_users (server_name);

-- ============================================================
-- Federation: Server signing keys (Ed25519)
-- ============================================================
CREATE TABLE IF NOT EXISTS server_signing_keys (
    id              TEXT PRIMARY KEY,
    key_id          TEXT NOT NULL UNIQUE,
    private_key     TEXT NOT NULL,
    public_key      TEXT NOT NULL,
    algorithm       TEXT NOT NULL DEFAULT 'ed25519',
    active          INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at      TEXT
);

-- ============================================================
-- Audit Log
-- ============================================================
CREATE TABLE IF NOT EXISTS audit_log (
    id          TEXT PRIMARY KEY,
    server_id   TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    action      TEXT NOT NULL,
    target_type TEXT,
    target_id   TEXT,
    changes     TEXT,          -- JSON
    reason      TEXT,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_audit_log_server ON audit_log (server_id, created_at DESC);
