-- ============================================================
-- Phase 16: Advanced Collaboration & Productivity (v1.5)
-- ============================================================

-- ── 16-01: Integrated Tasks & Checklists ──────────────────────────────────────

CREATE TABLE IF NOT EXISTS tasks (
    id              UUID PRIMARY KEY,
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    channel_id      UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    creator_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    assignee_id     UUID           REFERENCES users(id) ON DELETE SET NULL,
    title           TEXT NOT NULL,
    description     TEXT,
    status          TEXT NOT NULL DEFAULT 'open'
                    CHECK (status IN ('open', 'in_progress', 'done', 'cancelled')),
    priority        TEXT NOT NULL DEFAULT 'medium'
                    CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
    due_at          TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    position        INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tasks_channel   ON tasks (channel_id, status, position);
CREATE INDEX idx_tasks_server    ON tasks (server_id, status);
CREATE INDEX idx_tasks_assignee  ON tasks (assignee_id) WHERE assignee_id IS NOT NULL;
CREATE INDEX idx_tasks_due       ON tasks (due_at)       WHERE due_at IS NOT NULL AND status != 'done';

-- Task checklist items (sub-tasks)
CREATE TABLE IF NOT EXISTS task_checklist_items (
    id          UUID PRIMARY KEY,
    task_id     UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    content     TEXT NOT NULL,
    checked     BOOLEAN NOT NULL DEFAULT FALSE,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_checklist_task ON task_checklist_items (task_id, position);

-- Task reminders
CREATE TABLE IF NOT EXISTS task_reminders (
    id          UUID PRIMARY KEY,
    task_id     UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    remind_at   TIMESTAMPTZ NOT NULL,
    fired       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reminders_fire ON task_reminders (remind_at) WHERE fired = FALSE;

-- ── 16-02: Calendar Integration ───────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS calendar_events (
    id              UUID PRIMARY KEY,
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    channel_id      UUID           REFERENCES channels(id) ON DELETE SET NULL,
    creator_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           TEXT NOT NULL,
    description     TEXT,
    location        TEXT,
    starts_at       TIMESTAMPTZ NOT NULL,
    ends_at         TIMESTAMPTZ NOT NULL,
    all_day         BOOLEAN NOT NULL DEFAULT FALSE,
    -- Recurrence (RFC 5545 RRULE string, e.g. "FREQ=WEEKLY;BYDAY=MO,WE,FR")
    rrule           TEXT,
    -- Color tag for calendar UI
    color           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_cal_server   ON calendar_events (server_id, starts_at);
CREATE INDEX idx_cal_channel  ON calendar_events (channel_id, starts_at) WHERE channel_id IS NOT NULL;
CREATE INDEX idx_cal_range    ON calendar_events (starts_at, ends_at);

-- RSVP (reuses existing event_rsvps pattern but linked to calendar_events)
CREATE TABLE IF NOT EXISTS calendar_rsvps (
    event_id    UUID NOT NULL REFERENCES calendar_events(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id)           ON DELETE CASCADE,
    status      TEXT NOT NULL DEFAULT 'interested'
                CHECK (status IN ('going', 'interested', 'declined')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (event_id, user_id)
);

-- Calendar reminders
CREATE TABLE IF NOT EXISTS calendar_reminders (
    id          UUID PRIMARY KEY,
    event_id    UUID NOT NULL REFERENCES calendar_events(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    minutes_before INTEGER NOT NULL DEFAULT 15,
    fired       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_cal_reminders ON calendar_reminders (fired) WHERE fired = FALSE;

-- ── 16-03: File Versioning & History ──────────────────────────────────────────

CREATE TABLE IF NOT EXISTS file_versions (
    id              UUID PRIMARY KEY,
    attachment_id   UUID NOT NULL REFERENCES attachments(id) ON DELETE CASCADE,
    uploader_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    version_number  INTEGER NOT NULL DEFAULT 1,
    filename        TEXT NOT NULL,
    content_type    TEXT,
    size            BIGINT NOT NULL DEFAULT 0,
    storage_key     TEXT NOT NULL,
    sha256          TEXT,
    comment         TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_file_ver_unique ON file_versions (attachment_id, version_number);
CREATE INDEX idx_file_ver_attach        ON file_versions (attachment_id, version_number DESC);

-- Per-server storage quota tracking
CREATE TABLE IF NOT EXISTS server_storage_quotas (
    server_id       UUID PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    max_bytes       BIGINT NOT NULL DEFAULT 5368709120,  -- 5 GiB default
    used_bytes      BIGINT NOT NULL DEFAULT 0,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── 16-04: On-Device AI Assists ───────────────────────────────────────────────
-- Minimal server-side: only stores per-user opt-in preferences and
-- digest history.  Actual inference runs on-device (privacy-first).

CREATE TABLE IF NOT EXISTS ai_preferences (
    user_id             UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    summaries_enabled   BOOLEAN NOT NULL DEFAULT FALSE,
    smart_replies       BOOLEAN NOT NULL DEFAULT FALSE,
    auto_mod_suggest    BOOLEAN NOT NULL DEFAULT FALSE,
    digest_enabled      BOOLEAN NOT NULL DEFAULT FALSE,
    digest_interval     TEXT NOT NULL DEFAULT 'daily'
                        CHECK (digest_interval IN ('daily', 'weekly')),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Channel digest snapshots (server-generated summaries stored after on-device generation)
CREATE TABLE IF NOT EXISTS channel_digests (
    id          UUID PRIMARY KEY,
    channel_id  UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    period_start TIMESTAMPTZ NOT NULL,
    period_end   TIMESTAMPTZ NOT NULL,
    summary     TEXT NOT NULL,
    message_count INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_digests_channel ON channel_digests (channel_id, period_end DESC);
CREATE INDEX idx_digests_user    ON channel_digests (user_id, created_at DESC);
