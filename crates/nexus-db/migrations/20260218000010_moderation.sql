-- =============================================================================
-- Phase 9.8: Moderation & Safety
-- =============================================================================
-- 09.8-01  Server Audit Log  — fix existing table + fill gaps
-- 09.8-02  Timeout & Temp-Ban
-- 09.8-03  Message Reporting
-- 09.8-04  Content Filters
-- =============================================================================


-- ── 09.8-01  Audit Log ────────────────────────────────────────────────────────
-- The audit_log table was created in migration 00001 but declared user_id as
-- NOT NULL, which conflicts with the ON DELETE SET NULL referential action.
-- Drop the NOT NULL constraint so system-generated entries and user deletions
-- both work correctly.
ALTER TABLE audit_log ALTER COLUMN user_id DROP NOT NULL;


-- ── 09.8-02  User Timeouts ───────────────────────────────────────────────────
-- Stores timeout records with moderator attribution and a reason.
-- The members.communication_disabled_until column (from migration 00001)
-- is used for inline enforcement; this table tracks history and metadata.

CREATE TABLE user_timeouts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id)   ON DELETE CASCADE,
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    moderator_id    UUID             REFERENCES users(id) ON DELETE SET NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    reason          TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, server_id)
);

CREATE INDEX ix_user_timeouts_server  ON user_timeouts (server_id);
CREATE INDEX ix_user_timeouts_expires ON user_timeouts (expires_at);


-- ── 09.8-02  Temp-Bans ───────────────────────────────────────────────────────
-- Add an optional expiry date to the existing bans table.
-- expires_at = NULL means the ban is permanent.

ALTER TABLE bans ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS ix_bans_expires
    ON bans (expires_at)
    WHERE expires_at IS NOT NULL;


-- ── 09.8-03  Message Reports ─────────────────────────────────────────────────
-- A user can report a message; moderators review the queue.
-- A deduplication index prevents a single user from submitting more than one
-- pending report for the same message.

CREATE TABLE message_reports (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id          UUID NOT NULL,
    channel_id          UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    server_id           UUID          REFERENCES servers(id)  ON DELETE CASCADE,
    reporter_id         UUID NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    reason              TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'resolved', 'dismissed')),
    resolved_by         UUID          REFERENCES users(id)    ON DELETE SET NULL,
    resolved_at         TIMESTAMPTZ,
    resolution_action   TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ix_message_reports_server
    ON message_reports (server_id, status, created_at DESC);

-- One pending report per user per message
CREATE UNIQUE INDEX ix_message_reports_dedup
    ON message_reports (message_id, reporter_id)
    WHERE status = 'pending';


-- ── 09.8-04  Server Word Filters ─────────────────────────────────────────────
-- Stores per-server blocked word/phrase patterns.
-- action: block = reject message, delete = auto-delete after post, warn = allow + warn user

CREATE TABLE server_word_filters (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id   UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    pattern     TEXT NOT NULL,
    action      TEXT NOT NULL DEFAULT 'block'
                CHECK (action IN ('block', 'delete', 'warn')),
    created_by  UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (server_id, pattern)
);

CREATE INDEX ix_word_filters_server ON server_word_filters (server_id);
