-- Phase 22: Enhanced Voice & Real-Time Collaboration (v2.1)
-- Sub-phases: 22-01 Video Grid Polish, 22-02 Live Streaming,
--             22-03 Collaborative Sessions, 22-04 Spatial Audio,
--             22-05 Low-Latency & Quality

-- ── 22-01: Video Grid Polish ──────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS video_layouts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id      UUID NOT NULL,
    user_id         UUID NOT NULL,
    layout_type     TEXT NOT NULL DEFAULT 'gallery', -- gallery | spotlight | sidebar | custom
    pinned_users    JSONB NOT NULL DEFAULT '[]',
    custom_positions JSONB NOT NULL DEFAULT '{}',
    pip_enabled     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(channel_id, user_id)
);

CREATE TABLE IF NOT EXISTS virtual_backgrounds (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    name            TEXT NOT NULL,
    bg_type         TEXT NOT NULL DEFAULT 'blur',   -- blur | image | none
    image_url       TEXT,
    is_default      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── 22-02: Live Streaming ─────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS live_streams (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id      UUID NOT NULL,
    streamer_id     UUID NOT NULL,
    title           TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'starting', -- starting | live | ended
    viewer_count    INT  NOT NULL DEFAULT 0,
    max_viewers     INT  NOT NULL DEFAULT 0,
    is_e2ee         BOOLEAN NOT NULL DEFAULT FALSE,
    recording_url   TEXT,
    hls_url         TEXT,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    ended_at        TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_live_streams_channel ON live_streams(channel_id, status);

CREATE TABLE IF NOT EXISTS stream_viewers (
    stream_id       UUID NOT NULL REFERENCES live_streams(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL,
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    left_at         TIMESTAMPTZ,
    PRIMARY KEY (stream_id, user_id)
);

-- ── 22-03: Collaborative Sessions ─────────────────────────────────────────

CREATE TABLE IF NOT EXISTS breakout_rooms (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_channel  UUID NOT NULL,
    name            TEXT NOT NULL,
    capacity        INT  NOT NULL DEFAULT 10,
    status          TEXT NOT NULL DEFAULT 'open', -- open | closed | merged
    created_by      UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_at       TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS collab_sessions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id      UUID NOT NULL,
    session_type    TEXT NOT NULL DEFAULT 'whiteboard', -- whiteboard | document | screen_share
    document_id     UUID,
    participants    JSONB NOT NULL DEFAULT '[]',
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    ended_at        TIMESTAMPTZ
);

-- ── 22-04: Spatial & Immersive Audio ──────────────────────────────────────

CREATE TABLE IF NOT EXISTS spatial_audio_configs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id      UUID NOT NULL UNIQUE,
    preset          TEXT NOT NULL DEFAULT 'conference', -- conference | auditorium | campfire | custom
    room_width      REAL NOT NULL DEFAULT 10.0,
    room_depth      REAL NOT NULL DEFAULT 10.0,
    room_height     REAL NOT NULL DEFAULT 3.0,
    positions       JSONB NOT NULL DEFAULT '{}',       -- { user_id: { x, y, z } }
    hrtf_enabled    BOOLEAN NOT NULL DEFAULT TRUE,
    ambisonics_order INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── 22-05: Low-Latency & Quality ─────────────────────────────────────────

CREATE TABLE IF NOT EXISTS voice_presets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL UNIQUE,
    description     TEXT,
    target_latency_ms INT NOT NULL DEFAULT 80,
    jitter_buffer_ms  INT NOT NULL DEFAULT 50,
    fec_level       INT NOT NULL DEFAULT 1,         -- 0=off, 1=low, 2=high
    dtx_enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    normalization   BOOLEAN NOT NULL DEFAULT TRUE,
    is_builtin      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Seed built-in voice presets
INSERT INTO voice_presets (id, name, description, target_latency_ms, jitter_buffer_ms, fec_level, dtx_enabled, normalization, is_builtin)
VALUES
    (gen_random_uuid(), 'balanced',    'Default balanced preset',               80, 50, 1, TRUE,  TRUE,  TRUE),
    (gen_random_uuid(), 'competitive', 'Sub-50ms for gaming/esports',           30, 20, 2, FALSE, TRUE,  TRUE),
    (gen_random_uuid(), 'music',       'High fidelity for music/podcast',      120, 80, 0, FALSE, FALSE, TRUE),
    (gen_random_uuid(), 'low_bandwidth','Optimised for poor connections',       150,100, 2, TRUE,  TRUE,  TRUE)
ON CONFLICT (name) DO NOTHING;
