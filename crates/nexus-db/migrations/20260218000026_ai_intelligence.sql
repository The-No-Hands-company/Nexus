-- Phase 21: Advanced AI & Intelligence Layer (v2.0)
-- Sub-phases: 21-01 Semantic Search, 21-02 Proactive Assists,
--             21-03 Moderation AI, 21-04 Voice AI, 21-05 AI Governance

-- ── 21-01: Semantic Search ────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS search_embeddings (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id      UUID NOT NULL,
    channel_id      UUID NOT NULL,
    embedding       BYTEA,                          -- serialised float32 vector
    model_name      TEXT NOT NULL DEFAULT 'minilm',
    model_version   TEXT NOT NULL DEFAULT 'v1',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_search_embed_channel ON search_embeddings(channel_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_search_embed_msg ON search_embeddings(message_id);

CREATE TABLE IF NOT EXISTS search_queries (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    raw_query       TEXT NOT NULL,
    parsed_filters  JSONB NOT NULL DEFAULT '{}',
    result_count    INT  NOT NULL DEFAULT 0,
    latency_ms      INT  NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── 21-02: Proactive Assists ──────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS ai_suggestions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    channel_id      UUID NOT NULL,
    suggestion_type TEXT NOT NULL DEFAULT 'reply', -- reply | summary | tag | notification
    content         TEXT NOT NULL,
    context_ids     JSONB NOT NULL DEFAULT '[]',   -- message IDs that informed the suggestion
    model_name      TEXT NOT NULL DEFAULT 'local',
    accepted        BOOLEAN,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS thread_summaries (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_id       UUID NOT NULL,
    channel_id      UUID NOT NULL,
    summary         TEXT NOT NULL,
    message_count   INT  NOT NULL DEFAULT 0,
    model_name      TEXT NOT NULL DEFAULT 'local',
    model_version   TEXT NOT NULL DEFAULT 'v1',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_thread_summary ON thread_summaries(thread_id);

-- ── 21-03: Moderation AI ─────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS toxicity_scores (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id      UUID NOT NULL,
    server_id       UUID NOT NULL,
    score           REAL NOT NULL DEFAULT 0.0,   -- 0.0 (safe) .. 1.0 (toxic)
    categories      JSONB NOT NULL DEFAULT '{}', -- { "spam": 0.8, "harassment": 0.2 }
    model_name      TEXT NOT NULL DEFAULT 'local',
    flagged         BOOLEAN NOT NULL DEFAULT FALSE,
    reviewed        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_toxicity_server ON toxicity_scores(server_id, flagged);

CREATE TABLE IF NOT EXISTS raid_detections (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL,
    detection_type  TEXT NOT NULL DEFAULT 'join_spike', -- join_spike | spam_flood | coordinated
    severity        TEXT NOT NULL DEFAULT 'medium',     -- low | medium | high | critical
    details         JSONB NOT NULL DEFAULT '{}',
    auto_actions    JSONB NOT NULL DEFAULT '[]',        -- actions taken automatically
    resolved        BOOLEAN NOT NULL DEFAULT FALSE,
    detected_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at     TIMESTAMPTZ
);

-- ── 21-04: Voice AI ──────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS voice_transcripts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id      UUID NOT NULL,
    session_id      UUID,
    speaker_id      UUID,                            -- NULL if diarisation uncertain
    segment_start   REAL NOT NULL DEFAULT 0.0,       -- seconds from session start
    segment_end     REAL NOT NULL DEFAULT 0.0,
    text            TEXT NOT NULL,
    language        TEXT NOT NULL DEFAULT 'en',
    confidence      REAL NOT NULL DEFAULT 0.0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_voice_transcript_channel ON voice_transcripts(channel_id, created_at DESC);

CREATE TABLE IF NOT EXISTS voice_commands (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    channel_id      UUID NOT NULL,
    command_text    TEXT NOT NULL,
    action          TEXT NOT NULL,      -- mute | deafen | switch_channel | custom
    confidence      REAL NOT NULL DEFAULT 0.0,
    executed        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── 21-05: AI Governance ─────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS ai_consent (
    user_id         UUID NOT NULL,
    server_id       UUID NOT NULL,
    feature         TEXT NOT NULL,        -- semantic_search | suggestions | toxicity | voice_ai
    enabled         BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, server_id, feature)
);

CREATE TABLE IF NOT EXISTS ai_audit_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL,
    feature         TEXT NOT NULL,
    action          TEXT NOT NULL,        -- enabled | disabled | invoked | error
    actor_id        UUID,
    details         JSONB NOT NULL DEFAULT '{}',
    model_name      TEXT,
    model_version   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_ai_audit_server ON ai_audit_log(server_id, created_at DESC);
