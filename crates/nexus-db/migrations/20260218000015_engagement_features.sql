-- Migration 015: Engagement Features (Phase 13)
-- Adds polls, scheduled messages, bookmarks, drafts, disappearing messages,
-- note-to-self channels, and status auto-expiry.

-- ============================================================
-- Polls
-- Stand-alone polls embedded in a channel (optionally attached to a message).
-- ============================================================
CREATE TABLE IF NOT EXISTS polls (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id      UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    -- nullable: poll not yet posted as a message, or standalone
    message_id      UUID,
    author_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    question        TEXT NOT NULL,
    -- JSON array of option strings: ["Yes", "No", "Maybe"]
    options         JSONB NOT NULL DEFAULT '[]',
    ends_at         TIMESTAMPTZ,
    allow_multiselect BOOLEAN NOT NULL DEFAULT false,
    is_anonymous    BOOLEAN NOT NULL DEFAULT false,
    -- status: 'open' | 'ended'
    status          TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'ended')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_polls_channel  ON polls (channel_id);
CREATE INDEX IF NOT EXISTS idx_polls_ends_at  ON polls (ends_at) WHERE ends_at IS NOT NULL AND status = 'open';

CREATE TABLE IF NOT EXISTS poll_votes (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    poll_id      UUID NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    option_index INT NOT NULL,
    voted_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (poll_id, user_id, option_index)
);

CREATE INDEX IF NOT EXISTS idx_poll_votes_poll   ON poll_votes (poll_id);
CREATE INDEX IF NOT EXISTS idx_poll_votes_user   ON poll_votes (user_id);

-- ============================================================
-- Scheduled Messages
-- Messages queued to be sent at a future time.
-- ============================================================
CREATE TABLE IF NOT EXISTS scheduled_messages (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id   UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content      TEXT NOT NULL DEFAULT '',
    -- JSON array of attachment metadata objects
    attachments  JSONB NOT NULL DEFAULT '[]',
    scheduled_at TIMESTAMPTZ NOT NULL,
    -- status: 'pending' | 'sent' | 'cancelled'
    status       TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sent', 'cancelled')),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_scheduled_messages_channel  ON scheduled_messages (channel_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_messages_author   ON scheduled_messages (author_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_messages_dispatch ON scheduled_messages (scheduled_at)
    WHERE status = 'pending';

-- ============================================================
-- Message Bookmarks
-- Per-user saved messages with an optional personal note.
-- ============================================================
CREATE TABLE IF NOT EXISTS message_bookmarks (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message_id UUID NOT NULL,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    note       TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, message_id)
);

CREATE INDEX IF NOT EXISTS idx_message_bookmarks_user ON message_bookmarks (user_id, created_at DESC);

-- ============================================================
-- Disappearing Messages
-- Per-channel auto-delete timer (0 = off, >0 = seconds until deletion).
-- ============================================================
ALTER TABLE channels ADD COLUMN IF NOT EXISTS disappear_after_seconds INT NOT NULL DEFAULT 0;

-- expires_at on messages — set when a channel has disappear_after_seconds > 0
ALTER TABLE messages ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_messages_expires_at ON messages (expires_at)
    WHERE expires_at IS NOT NULL;

-- ============================================================
-- Message Drafts
-- Per-user per-channel draft (one draft per (user, channel) pair).
-- ============================================================
CREATE TABLE IF NOT EXISTS message_drafts (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id   UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    content      TEXT NOT NULL DEFAULT '',
    -- JSON array of attachment stubs
    attachments  JSONB NOT NULL DEFAULT '[]',
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, channel_id)
);

CREATE INDEX IF NOT EXISTS idx_message_drafts_user ON message_drafts (user_id);

-- ============================================================
-- Note-to-Self Channel
-- Materialised as a row in the channels table (type = 'dm').
-- We track the association here so we can surface it efficiently.
-- ============================================================
CREATE TABLE IF NOT EXISTS note_to_self_channels (
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE PRIMARY KEY,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE
);

-- ============================================================
-- Status Auto-Expiry
-- Optional expiry timestamp for a user's custom status.
-- ============================================================
ALTER TABLE users ADD COLUMN IF NOT EXISTS custom_status_expires_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_users_status_expires ON users (custom_status_expires_at)
    WHERE custom_status_expires_at IS NOT NULL;
