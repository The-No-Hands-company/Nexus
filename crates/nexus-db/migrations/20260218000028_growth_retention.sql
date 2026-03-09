-- Phase 23: User Growth & Retention (v2.2)
-- Sub-phases: 23-01 Onboarding Personalization, 23-02 Cross-Device Continuity,
--             23-03 Gamification Lite, 23-04 Offline-First

-- ── 23-01: Onboarding Personalization ─────────────────────────────────────

CREATE TABLE IF NOT EXISTS server_recommendations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    server_id       UUID NOT NULL,
    score           REAL NOT NULL DEFAULT 0.0,
    reason          TEXT NOT NULL DEFAULT 'interest', -- interest | friend | trending | activity
    dismissed       BOOLEAN NOT NULL DEFAULT FALSE,
    joined          BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, server_id)
);

CREATE TABLE IF NOT EXISTS onboarding_flows (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL,
    steps           JSONB NOT NULL DEFAULT '[]',
    adaptive        BOOLEAN NOT NULL DEFAULT TRUE,
    skip_completed  BOOLEAN NOT NULL DEFAULT TRUE,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(server_id)
);

-- ── 23-02: Cross-Device Continuity ────────────────────────────────────────

CREATE TABLE IF NOT EXISTS device_sessions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    device_id       TEXT NOT NULL,
    device_type     TEXT NOT NULL DEFAULT 'desktop', -- desktop | mobile | web
    last_channel_id UUID,
    scroll_position JSONB,                           -- { channel_id: position }
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    last_seen_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, device_id)
);

CREATE TABLE IF NOT EXISTS clipboard_sync (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    source_device   TEXT NOT NULL,
    content_type    TEXT NOT NULL DEFAULT 'text', -- text | image | file_ref
    encrypted_data  BYTEA NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_clipboard_user ON clipboard_sync(user_id, created_at DESC);

-- ── 23-03: Gamification Lite ──────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_xp (
    user_id         UUID NOT NULL,
    server_id       UUID NOT NULL,
    xp              BIGINT NOT NULL DEFAULT 0,
    level           INT    NOT NULL DEFAULT 0,
    last_xp_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, server_id)
);

CREATE TABLE IF NOT EXISTS gamification_configs (
    server_id       UUID PRIMARY KEY,
    enabled         BOOLEAN NOT NULL DEFAULT FALSE,
    xp_per_message  INT NOT NULL DEFAULT 10,
    xp_per_reaction INT NOT NULL DEFAULT 2,
    xp_per_voice_min INT NOT NULL DEFAULT 5,
    level_formula   TEXT NOT NULL DEFAULT 'linear', -- linear | quadratic | custom
    streak_enabled  BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS achievements (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT,
    icon_url        TEXT,
    criteria        JSONB NOT NULL DEFAULT '{}',  -- { "type": "message_count", "threshold": 1000 }
    reward_xp       INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS user_achievements (
    user_id         UUID NOT NULL,
    achievement_id  UUID NOT NULL REFERENCES achievements(id) ON DELETE CASCADE,
    earned_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, achievement_id)
);

CREATE TABLE IF NOT EXISTS activity_streaks (
    user_id         UUID NOT NULL,
    server_id       UUID NOT NULL,
    current_streak  INT  NOT NULL DEFAULT 0,
    longest_streak  INT  NOT NULL DEFAULT 0,
    last_active_date DATE NOT NULL DEFAULT CURRENT_DATE,
    PRIMARY KEY (user_id, server_id)
);

-- ── 23-04: Offline-First ─────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS sync_cursors (
    user_id         UUID NOT NULL,
    device_id       TEXT NOT NULL,
    channel_id      UUID NOT NULL,
    last_message_id UUID,
    last_synced_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, device_id, channel_id)
);

CREATE TABLE IF NOT EXISTS offline_queue (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    device_id       TEXT NOT NULL,
    action_type     TEXT NOT NULL, -- send_message | edit_message | delete_message | react
    payload         JSONB NOT NULL DEFAULT '{}',
    status          TEXT NOT NULL DEFAULT 'pending', -- pending | synced | conflict
    conflict_data   JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    synced_at       TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_offline_queue_user ON offline_queue(user_id, device_id, status);
