-- Phase 17: Multimedia & Expression Enhancements (v1.6)
--
-- 17-01  Voice & Video Notes
-- 17-02  Stories & Ephemeral Status
-- 17-03  In-Chat Drawing & Annotation
-- 17-04  Advanced Voice Features (config only — spatial audio, noise gate)
-- 17-05  Media Gallery (view over existing attachments — config + saved filters)

-- ═══════════════════════════════════════════════════════════════════════
-- 17-01: Voice & Video Notes
-- ═══════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS voice_notes (
    id            UUID PRIMARY KEY,
    channel_id    UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- audio storage
    storage_key   TEXT NOT NULL,               -- S3/MinIO object key
    filename      TEXT NOT NULL DEFAULT 'voice.opus',
    content_type  TEXT NOT NULL DEFAULT 'audio/opus',
    size          BIGINT NOT NULL DEFAULT 0,
    duration_ms   INT NOT NULL DEFAULT 0,      -- milliseconds
    waveform      TEXT,                        -- JSON array of 0-1 floats for preview
    -- optional transcript (generated on-device)
    transcript    TEXT,
    -- linked to a message
    message_id    UUID REFERENCES messages(id) ON DELETE SET NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_voice_notes_channel ON voice_notes(channel_id, created_at DESC);
CREATE INDEX idx_voice_notes_author  ON voice_notes(author_id, created_at DESC);

CREATE TABLE IF NOT EXISTS video_notes (
    id            UUID PRIMARY KEY,
    channel_id    UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- video storage
    storage_key   TEXT NOT NULL,
    filename      TEXT NOT NULL DEFAULT 'video.webm',
    content_type  TEXT NOT NULL DEFAULT 'video/webm',
    size          BIGINT NOT NULL DEFAULT 0,
    duration_ms   INT NOT NULL DEFAULT 0,
    width         INT,
    height        INT,
    thumbnail_key TEXT,                        -- S3 key for frame thumbnail
    -- optional transcript
    transcript    TEXT,
    message_id    UUID REFERENCES messages(id) ON DELETE SET NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_video_notes_channel ON video_notes(channel_id, created_at DESC);

-- ═══════════════════════════════════════════════════════════════════════
-- 17-02: Stories & Ephemeral Status
-- ═══════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS stories (
    id            UUID PRIMARY KEY,
    author_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- content
    media_type    TEXT NOT NULL CHECK (media_type IN ('image', 'video', 'text')),
    storage_key   TEXT,                        -- S3 key (null for text-only)
    text_content  TEXT,                        -- text overlay / text-only body
    text_style    JSONB,                       -- {"font","color","bg","align"}
    -- lifecycle
    expires_at    TIMESTAMPTZ NOT NULL,        -- auto-purge after this
    -- privacy
    visibility    TEXT NOT NULL DEFAULT 'friends'
                  CHECK (visibility IN ('friends', 'server', 'everyone')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_stories_author  ON stories(author_id, created_at DESC);
CREATE INDEX idx_stories_expires ON stories(expires_at);

CREATE TABLE IF NOT EXISTS story_views (
    story_id    UUID NOT NULL REFERENCES stories(id) ON DELETE CASCADE,
    viewer_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    viewed_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (story_id, viewer_id)
);

-- ═══════════════════════════════════════════════════════════════════════
-- 17-03: In-Chat Drawing & Annotation
-- ═══════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS drawings (
    id            UUID PRIMARY KEY,
    channel_id    UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- content
    drawing_data  JSONB NOT NULL,              -- strokes / shapes JSON (vector format)
    width         INT NOT NULL DEFAULT 800,
    height        INT NOT NULL DEFAULT 600,
    -- optional rasterised snapshot for non-interactive preview
    preview_key   TEXT,                        -- S3 key for PNG preview
    -- linked to a message, or standalone whiteboard
    message_id    UUID REFERENCES messages(id) ON DELETE SET NULL,
    is_whiteboard BOOLEAN NOT NULL DEFAULT FALSE,  -- voice-channel shared whiteboard
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_drawings_channel ON drawings(channel_id, created_at DESC);

-- ═══════════════════════════════════════════════════════════════════════
-- 17-04: Advanced Voice Features (per-user / per-channel config)
-- ═══════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS voice_settings (
    user_id             UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    spatial_audio       BOOLEAN NOT NULL DEFAULT FALSE,
    noise_gate_enabled  BOOLEAN NOT NULL DEFAULT TRUE,
    noise_gate_threshold REAL NOT NULL DEFAULT -50.0,   -- dBFS
    echo_cancel_level   TEXT NOT NULL DEFAULT 'moderate'
                        CHECK (echo_cancel_level IN ('off', 'low', 'moderate', 'aggressive')),
    auto_gain           BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Per-channel background music queue (bot-controlled, optional)
CREATE TABLE IF NOT EXISTS voice_music_queue (
    id          UUID PRIMARY KEY,
    channel_id  UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    added_by    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    source_url  TEXT NOT NULL,                 -- bot-resolved audio URL
    duration_ms INT NOT NULL DEFAULT 0,
    position    INT NOT NULL DEFAULT 0,
    status      TEXT NOT NULL DEFAULT 'queued'
                CHECK (status IN ('queued', 'playing', 'played', 'skipped')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_voice_music_channel ON voice_music_queue(channel_id, position);

-- ═══════════════════════════════════════════════════════════════════════
-- 17-05: Media Gallery (saved filters for existing attachments)
-- ═══════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS media_gallery_filters (
    id            UUID PRIMARY KEY,
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id     UUID REFERENCES servers(id) ON DELETE CASCADE,
    channel_id    UUID REFERENCES channels(id) ON DELETE CASCADE,
    -- filter criteria
    name          TEXT NOT NULL DEFAULT 'Untitled',
    media_types   TEXT[] NOT NULL DEFAULT '{"image","video","audio","file"}',
    date_from     TIMESTAMPTZ,
    date_to       TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gallery_filters_user ON media_gallery_filters(user_id);
