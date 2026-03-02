-- Migration: v0.14 Platform Differentiation
-- Message forwarding, Server Events, Sticker Packs, Stream/Topic threading.

-- ─── 14-01: Message Forwarding ───────────────────────────────────────────────

ALTER TABLE messages
    ADD COLUMN IF NOT EXISTS forwarded_from_message_id  UUID    REFERENCES messages(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS forwarded_from_channel_id  UUID    REFERENCES channels(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_messages_forwarded ON messages (forwarded_from_message_id)
    WHERE forwarded_from_message_id IS NOT NULL;

-- ─── 14-02: Server Events ────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS server_events (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID        NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    creator_id      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           TEXT        NOT NULL CHECK (char_length(title) BETWEEN 1 AND 100),
    description     TEXT        CHECK (char_length(description) <= 1000),
    starts_at       TIMESTAMPTZ NOT NULL,
    ends_at         TIMESTAMPTZ,
    location        TEXT        CHECK (char_length(location) <= 200),
    channel_id      UUID        REFERENCES channels(id) ON DELETE SET NULL,
    cover_image     TEXT,
    -- scheduled | active | completed | cancelled
    status          TEXT        NOT NULL DEFAULT 'scheduled'
                        CHECK (status IN ('scheduled','active','completed','cancelled')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_server_events_server  ON server_events (server_id, status, starts_at);
CREATE INDEX IF NOT EXISTS idx_server_events_fire    ON server_events (starts_at)
    WHERE status = 'scheduled';

-- RSVP — per-user interest in a server event
CREATE TABLE IF NOT EXISTS server_event_rsvps (
    event_id    UUID        NOT NULL REFERENCES server_events(id) ON DELETE CASCADE,
    user_id     UUID        NOT NULL REFERENCES users(id)         ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (event_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_event_rsvps_user ON server_event_rsvps (user_id);

-- ─── 14-03: Sticker Packs ────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS sticker_packs (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT        NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    description     TEXT        CHECK (char_length(description) <= 200),
    cover_sticker_id UUID,                         -- FK set after stickers exist
    server_id       UUID        REFERENCES servers(id) ON DELETE CASCADE,   -- NULL = global pack
    is_premium      BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS stickers (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    pack_id     UUID        NOT NULL REFERENCES sticker_packs(id) ON DELETE CASCADE,
    server_id   UUID        REFERENCES servers(id) ON DELETE CASCADE,
    name        TEXT        NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    description TEXT        CHECK (char_length(description) <= 200),
    asset_url   TEXT        NOT NULL,
    -- png | apng | lottie
    type        TEXT        NOT NULL DEFAULT 'png' CHECK (type IN ('png','apng','lottie')),
    creator_id  UUID        REFERENCES users(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE sticker_packs
    ADD CONSTRAINT fk_cover_sticker
        FOREIGN KEY (cover_sticker_id) REFERENCES stickers(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_stickers_pack    ON stickers (pack_id);
CREATE INDEX IF NOT EXISTS idx_stickers_server  ON stickers (server_id);

-- Sticker sends on messages
ALTER TABLE messages
    ADD COLUMN IF NOT EXISTS sticker_ids UUID[] NOT NULL DEFAULT '{}';

-- ─── 14-04: Inline Bot trigger declarations ───────────────────────────────────

-- Each bot may declare zero or more inline trigger prefixes (e.g. "/gif")
CREATE TABLE IF NOT EXISTS bot_inline_triggers (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    bot_id      UUID        NOT NULL REFERENCES bot_applications(id) ON DELETE CASCADE,
    prefix      TEXT        NOT NULL CHECK (char_length(prefix) BETWEEN 1 AND 32),
    description TEXT        CHECK (char_length(description) <= 100),
    UNIQUE (bot_id, prefix)
);

CREATE INDEX IF NOT EXISTS idx_inline_triggers_prefix ON bot_inline_triggers (prefix);

-- ─── 14-05: Stream / Topic threading ─────────────────────────────────────────

ALTER TABLE channels
    ADD COLUMN IF NOT EXISTS is_stream BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE messages
    ADD COLUMN IF NOT EXISTS topic TEXT CHECK (char_length(topic) <= 80);

CREATE INDEX IF NOT EXISTS idx_messages_stream_topic ON messages (channel_id, topic)
    WHERE topic IS NOT NULL;
