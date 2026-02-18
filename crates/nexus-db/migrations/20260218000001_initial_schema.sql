-- Nexus Database Schema
-- Migration: Initial schema for users, servers, channels, roles, members, invites
-- Generated for PostgreSQL 15+

-- ============================================================================
-- Custom Types
-- ============================================================================

CREATE TYPE user_presence AS ENUM ('online', 'idle', 'do_not_disturb', 'invisible', 'offline');
CREATE TYPE channel_type AS ENUM ('text', 'voice', 'category', 'dm', 'group_dm', 'thread', 'forum', 'stage', 'announcement');

-- ============================================================================
-- Users
-- ============================================================================

CREATE TABLE users (
    id              UUID PRIMARY KEY,
    username        VARCHAR(32) NOT NULL UNIQUE,
    display_name    VARCHAR(64),
    email           VARCHAR(255) UNIQUE,
    password_hash   TEXT NOT NULL,
    avatar          TEXT,
    banner          TEXT,
    bio             VARCHAR(190),
    status          VARCHAR(128),
    presence        user_presence NOT NULL DEFAULT 'offline',
    flags           BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_username_lower ON users (LOWER(username));
CREATE INDEX idx_users_email_lower ON users (LOWER(email)) WHERE email IS NOT NULL;
CREATE INDEX idx_users_created_at ON users (created_at);

-- ============================================================================
-- Servers (Guilds)
-- ============================================================================

CREATE TABLE servers (
    id              UUID PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    description     VARCHAR(1000),
    icon            TEXT,
    banner          TEXT,
    owner_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    region          VARCHAR(32),
    is_public       BOOLEAN NOT NULL DEFAULT false,
    features        JSONB NOT NULL DEFAULT '{}',
    settings        JSONB NOT NULL DEFAULT '{}',
    vanity_code     VARCHAR(32) UNIQUE,
    member_count    INTEGER NOT NULL DEFAULT 0,
    max_file_size   BIGINT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_servers_owner ON servers (owner_id);
CREATE INDEX idx_servers_public ON servers (is_public, member_count DESC) WHERE is_public = true;
CREATE INDEX idx_servers_vanity ON servers (vanity_code) WHERE vanity_code IS NOT NULL;

-- ============================================================================
-- Channels
-- ============================================================================

CREATE TABLE channels (
    id                      UUID PRIMARY KEY,
    server_id               UUID REFERENCES servers(id) ON DELETE CASCADE,
    parent_id               UUID REFERENCES channels(id) ON DELETE SET NULL,
    channel_type            channel_type NOT NULL,
    name                    VARCHAR(100),
    topic                   VARCHAR(1024),
    position                INTEGER NOT NULL DEFAULT 0,
    nsfw                    BOOLEAN NOT NULL DEFAULT false,
    rate_limit_per_user     INTEGER NOT NULL DEFAULT 0,
    bitrate                 INTEGER,
    user_limit              INTEGER,
    encrypted               BOOLEAN NOT NULL DEFAULT false,
    permission_overwrites   JSONB NOT NULL DEFAULT '[]',
    last_message_id         UUID,
    auto_archive_duration   INTEGER,
    archived                BOOLEAN NOT NULL DEFAULT false,
    locked                  BOOLEAN NOT NULL DEFAULT false,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_channels_server ON channels (server_id, position);
CREATE INDEX idx_channels_parent ON channels (parent_id);
CREATE INDEX idx_channels_type ON channels (channel_type);

-- ============================================================================
-- DM Participants (for DM and Group DM channels)
-- ============================================================================

CREATE TABLE dm_participants (
    channel_id      UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (channel_id, user_id)
);

CREATE INDEX idx_dm_participants_user ON dm_participants (user_id);

-- ============================================================================
-- Roles
-- ============================================================================

CREATE TABLE roles (
    id              UUID PRIMARY KEY,
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name            VARCHAR(100) NOT NULL,
    color           INTEGER,
    hoist           BOOLEAN NOT NULL DEFAULT false,
    icon            TEXT,
    position        INTEGER NOT NULL DEFAULT 0,
    permissions     BIGINT NOT NULL DEFAULT 0,
    mentionable     BOOLEAN NOT NULL DEFAULT true,
    is_default      BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_roles_server ON roles (server_id, position DESC);
CREATE UNIQUE INDEX idx_roles_default ON roles (server_id) WHERE is_default = true;

-- ============================================================================
-- Members (User ↔ Server relationship)
-- ============================================================================

CREATE TABLE members (
    user_id                         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id                       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    nickname                        VARCHAR(32),
    avatar                          TEXT,
    roles                           UUID[] NOT NULL DEFAULT ARRAY[]::UUID[],
    muted                           BOOLEAN NOT NULL DEFAULT false,
    deafened                        BOOLEAN NOT NULL DEFAULT false,
    joined_at                       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    communication_disabled_until    TIMESTAMPTZ,
    PRIMARY KEY (user_id, server_id)
);

CREATE INDEX idx_members_server ON members (server_id, joined_at);
CREATE INDEX idx_members_user ON members (user_id);

-- ============================================================================
-- Invites
-- ============================================================================

CREATE TABLE invites (
    code            VARCHAR(16) PRIMARY KEY,
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    channel_id      UUID REFERENCES channels(id) ON DELETE SET NULL,
    inviter_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    max_uses        INTEGER,
    uses            INTEGER NOT NULL DEFAULT 0,
    expires_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invites_server ON invites (server_id);
CREATE INDEX idx_invites_expires ON invites (expires_at) WHERE expires_at IS NOT NULL;

-- ============================================================================
-- Refresh Tokens (for session management)
-- ============================================================================

CREATE TABLE refresh_tokens (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash      TEXT NOT NULL UNIQUE,
    device_info     TEXT,
    ip_address      INET,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_user ON refresh_tokens (user_id);
CREATE INDEX idx_refresh_tokens_expires ON refresh_tokens (expires_at);

-- ============================================================================
-- Audit Log
-- ============================================================================

CREATE TABLE audit_log (
    id              UUID PRIMARY KEY,
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    action          VARCHAR(64) NOT NULL,
    target_type     VARCHAR(32),
    target_id       UUID,
    changes         JSONB,
    reason          TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_log_server ON audit_log (server_id, created_at DESC);
CREATE INDEX idx_audit_log_user ON audit_log (user_id);

-- ============================================================================
-- Bans
-- ============================================================================

CREATE TABLE bans (
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    reason          TEXT,
    banned_by       UUID NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, server_id)
);

-- ============================================================================
-- Emoji (custom server emoji)
-- ============================================================================

CREATE TABLE emojis (
    id              UUID PRIMARY KEY,
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name            VARCHAR(32) NOT NULL,
    image_key       TEXT NOT NULL,
    creator_id      UUID REFERENCES users(id) ON DELETE SET NULL,
    animated        BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_emojis_server ON emojis (server_id);
