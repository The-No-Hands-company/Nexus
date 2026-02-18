-- v0.2 Chat MVP: Messages, reactions, read states
-- PostgreSQL for MVP (ScyllaDB migration planned for scale)

-- ============================================================================
-- Messages — the core of chat
-- ============================================================================
CREATE TABLE IF NOT EXISTS messages (
    id          UUID PRIMARY KEY,
    channel_id  UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id   UUID NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    content     TEXT NOT NULL DEFAULT '',
    message_type INTEGER NOT NULL DEFAULT 0,
    -- 0 = Default, 1 = RecipientAdd, 2 = RecipientRemove, 3 = Call,
    -- 4 = ChannelName, 5 = ChannelIcon, 6 = PinnedMessage, 7 = ServerJoin

    -- Edit tracking
    edited      BOOLEAN NOT NULL DEFAULT FALSE,
    edited_at   TIMESTAMPTZ,

    -- Pin
    pinned      BOOLEAN NOT NULL DEFAULT FALSE,

    -- Rich content (stored as JSONB for flexibility)
    embeds      JSONB NOT NULL DEFAULT '[]'::jsonb,
    attachments JSONB NOT NULL DEFAULT '[]'::jsonb,

    -- Mentions
    mentions        UUID[] NOT NULL DEFAULT '{}',
    mention_roles   UUID[] NOT NULL DEFAULT '{}',
    mention_everyone BOOLEAN NOT NULL DEFAULT FALSE,

    -- Reply / thread reference
    reference_message_id UUID REFERENCES messages(id) ON DELETE SET NULL,
    reference_channel_id UUID REFERENCES channels(id) ON DELETE SET NULL,
    thread_id            UUID REFERENCES channels(id) ON DELETE SET NULL,

    -- Flags
    flags       INTEGER NOT NULL DEFAULT 0,
    -- 1 = CROSSPOSTED, 2 = IS_CROSSPOST, 4 = SUPPRESS_EMBEDS, 8 = URGENT, 16 = EPHEMERAL

    -- Timestamps
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Fast lookups: messages in a channel ordered by time (cursor pagination)
CREATE INDEX idx_messages_channel_created ON messages (channel_id, created_at DESC);

-- Lookup by author (for user message history, moderation)
CREATE INDEX idx_messages_author ON messages (author_id, created_at DESC);

-- Pinned messages per channel
CREATE INDEX idx_messages_pinned ON messages (channel_id) WHERE pinned = TRUE;

-- Reference lookups (replies)
CREATE INDEX idx_messages_reference ON messages (reference_message_id) WHERE reference_message_id IS NOT NULL;

-- ============================================================================
-- Reactions — emoji reactions on messages
-- ============================================================================
CREATE TABLE IF NOT EXISTS reactions (
    message_id  UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji       TEXT NOT NULL,
    -- emoji is either a unicode emoji (e.g., "👍") or a custom emoji ID string (e.g., "custom:emoji_id")
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (message_id, user_id, emoji)
);

-- All reactions on a message (grouped by emoji)
CREATE INDEX idx_reactions_message ON reactions (message_id, emoji);

-- All reactions by a user (for cleanup on leave)
CREATE INDEX idx_reactions_user ON reactions (user_id);

-- ============================================================================
-- Read States — track where each user has read up to per channel
-- ============================================================================
CREATE TABLE IF NOT EXISTS read_states (
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id          UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    last_read_message_id UUID REFERENCES messages(id) ON DELETE SET NULL,
    mention_count       INTEGER NOT NULL DEFAULT 0,
    last_read_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (user_id, channel_id)
);

-- Lookup all read states for a user (for READY payload)
CREATE INDEX idx_read_states_user ON read_states (user_id);

-- ============================================================================
-- Update channels.last_message_id via trigger
-- ============================================================================
CREATE OR REPLACE FUNCTION update_channel_last_message()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE channels SET last_message_id = NEW.id, updated_at = NOW()
    WHERE id = NEW.channel_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_update_channel_last_message
    AFTER INSERT ON messages
    FOR EACH ROW
    EXECUTE FUNCTION update_channel_last_message();

-- ============================================================================
-- Full-text search (GIN index for PostgreSQL native search, MeiliSearch later)
-- ============================================================================
ALTER TABLE messages ADD COLUMN IF NOT EXISTS search_vector tsvector
    GENERATED ALWAYS AS (to_tsvector('english', coalesce(content, ''))) STORED;

CREATE INDEX idx_messages_search ON messages USING GIN (search_vector);
