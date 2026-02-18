-- Migration 003: Rich Features (v0.4)
-- Adds: attachments, threads, custom emoji, user_activities, presence extensions,
--       embed_cache, link_previews, and MeiliSearch sync queue.

-- ============================================================
-- Attachments — uploaded files linked to messages
-- ============================================================
CREATE TABLE IF NOT EXISTS attachments (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    uploader_id     UUID NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    server_id       UUID REFERENCES servers(id) ON DELETE SET NULL,
    channel_id      UUID REFERENCES channels(id) ON DELETE SET NULL,
    message_id      UUID REFERENCES messages(id) ON DELETE SET NULL,

    -- Original filename from the upload
    filename        TEXT NOT NULL,

    -- MIME type detected server-side (never trust client)
    content_type    TEXT NOT NULL,

    -- Size in bytes
    size            BIGINT NOT NULL,

    -- Storage key in MinIO/S3 (prefix/uuid.ext format)
    storage_key     TEXT NOT NULL UNIQUE,

    -- CDN/presigned URL (cached — regenerated on access)
    url             TEXT,

    -- Image/video metadata
    width           INT,
    height          INT,
    duration_secs   FLOAT,

    -- Whether this is marked as a spoiler (blur until clicked)
    spoiler         BOOLEAN NOT NULL DEFAULT false,

    -- Blurhash for progressive image loading (like Discord's)
    blurhash        TEXT,

    -- Hash for deduplication (SHA-256)
    sha256          TEXT,

    -- Upload state: pending -> processing -> ready | failed
    status          TEXT NOT NULL DEFAULT 'pending',

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_attachments_message   ON attachments (message_id) WHERE message_id IS NOT NULL;
CREATE INDEX idx_attachments_channel   ON attachments (channel_id, created_at DESC) WHERE channel_id IS NOT NULL;
CREATE INDEX idx_attachments_uploader  ON attachments (uploader_id, created_at DESC);
CREATE INDEX idx_attachments_storage   ON attachments (storage_key);

-- ============================================================
-- Custom server emoji
-- ============================================================
CREATE TABLE IF NOT EXISTS server_emoji (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    creator_id      UUID REFERENCES users(id) ON DELETE SET NULL,

    -- Short name used in :name: syntax
    name            TEXT NOT NULL,

    -- Storage key for emoji image
    storage_key     TEXT NOT NULL,
    url             TEXT,

    -- Whether this emoji is animated (GIF)
    animated        BOOLEAN NOT NULL DEFAULT false,

    -- Whether the emoji requires "managed" role (for bots etc.)
    managed         BOOLEAN NOT NULL DEFAULT false,

    -- Whether emoji is available (can be disabled if server loses perks)
    available       BOOLEAN NOT NULL DEFAULT true,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (server_id, name)
);

CREATE INDEX idx_server_emoji_server ON server_emoji (server_id);

-- ============================================================
-- Threads — spawned from messages, are themselves channels
-- ============================================================
-- Threads reuse the existing `channels` table (channel_type = 'thread').
-- This table stores the thread-specific metadata.
CREATE TABLE IF NOT EXISTS threads (
    -- The channel ID for this thread (FK to channels)
    channel_id          UUID PRIMARY KEY REFERENCES channels(id) ON DELETE CASCADE,

    -- The message that spawned this thread (optional for forum posts)
    parent_message_id   UUID REFERENCES messages(id) ON DELETE SET NULL,

    -- Owner of the thread (usually the person who created it)
    owner_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Thread title (displayed above the thread, can differ from channel name)
    title               TEXT NOT NULL,

    -- Message count in thread
    message_count       INT NOT NULL DEFAULT 0,

    -- How many members have joined the thread
    member_count        INT NOT NULL DEFAULT 0,

    -- Auto-archive after this many minutes of inactivity
    -- Values: 60, 1440 (day), 4320 (3 days), 10080 (week)
    auto_archive_minutes INT NOT NULL DEFAULT 1440,

    -- Whether the thread is archived
    archived            BOOLEAN NOT NULL DEFAULT false,

    -- When the thread was archived
    archived_at         TIMESTAMPTZ,

    -- Whether moderators have locked this thread (no new messages)
    locked              BOOLEAN NOT NULL DEFAULT false,

    -- Tags for forum-style channels
    tags                TEXT[] NOT NULL DEFAULT '{}',

    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_threads_parent_message ON threads (parent_message_id) WHERE parent_message_id IS NOT NULL;
CREATE INDEX idx_threads_owner          ON threads (owner_id);
CREATE INDEX idx_threads_archived       ON threads (archived, updated_at DESC);

-- Thread member tracking (users who have joined the thread)
CREATE TABLE IF NOT EXISTS thread_members (
    thread_id       UUID NOT NULL REFERENCES threads(channel_id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_viewed     TIMESTAMPTZ,
    PRIMARY KEY (thread_id, user_id)
);

CREATE INDEX idx_thread_members_user ON thread_members (user_id);

-- ============================================================
-- Link embed cache — cached link previews (Open Graph, oEmbed)
-- ============================================================
CREATE TABLE IF NOT EXISTS embed_cache (
    url             TEXT PRIMARY KEY,
    title           TEXT,
    description     TEXT,
    site_name       TEXT,
    image_url       TEXT,
    image_width     INT,
    image_height    INT,
    video_url       TEXT,
    embed_type      TEXT NOT NULL DEFAULT 'link',  -- 'link', 'image', 'video', 'rich'
    color           INT,
    provider_name   TEXT,
    author_name     TEXT,
    author_url      TEXT,
    -- When to re-fetch (TTL-based refresh)
    expires_at      TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '24 hours'),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_embed_cache_expires ON embed_cache (expires_at);

-- ============================================================
-- User activities — rich presence (game, listening, streaming)
-- ============================================================
CREATE TABLE IF NOT EXISTS user_activities (
    user_id         UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,

    -- Activity type: 'playing', 'streaming', 'listening', 'watching', 'competing'
    activity_type   TEXT,

    -- Activity name (e.g. "Minecraft", "Spotify", "Twitch")
    name            TEXT,

    -- Detail line (e.g. track name, game map)
    details         TEXT,

    -- State line (e.g. "In a party of 2", "Paused")
    state           TEXT,

    -- Large image key or URL
    large_image     TEXT,

    -- Small image key or URL
    small_image     TEXT,

    -- Stream URL (for type = 'streaming')
    url             TEXT,

    -- Application ID (for bot-reported activities)
    application_id  UUID,

    -- Activity timestamps
    started_at      TIMESTAMPTZ,
    ends_at         TIMESTAMPTZ,

    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- MeiliSearch sync queue — async indexing jobs
-- ============================================================
CREATE TABLE IF NOT EXISTS search_sync_queue (
    id              BIGSERIAL PRIMARY KEY,
    operation       TEXT NOT NULL,   -- 'index', 'update', 'delete'
    index_name      TEXT NOT NULL,   -- 'messages', 'servers', 'users'
    document_id     TEXT NOT NULL,
    payload         JSONB,
    processed       BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_search_sync_queue_unprocessed ON search_sync_queue (created_at)
    WHERE processed = false;

-- ============================================================
-- Extend messages table: link thread_id to threads, add attachment_ids
-- ============================================================
ALTER TABLE messages
    ADD COLUMN IF NOT EXISTS attachment_ids UUID[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS search_indexed BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX IF NOT EXISTS idx_messages_not_indexed ON messages (created_at)
    WHERE search_indexed = false;

-- ============================================================
-- Extend channels: add thread_metadata fields to channels record
-- ============================================================
ALTER TABLE channels
    ADD COLUMN IF NOT EXISTS thread_message_count INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS thread_member_count  INT NOT NULL DEFAULT 0;

-- ============================================================
-- Extend users: custom status with emoji
-- ============================================================
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS custom_status_emoji TEXT,
    ADD COLUMN IF NOT EXISTS custom_status_expires_at TIMESTAMPTZ;

-- Auto-update updated_at on attachments
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_attachments_updated_at
    BEFORE UPDATE ON attachments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_threads_updated_at
    BEFORE UPDATE ON threads
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
