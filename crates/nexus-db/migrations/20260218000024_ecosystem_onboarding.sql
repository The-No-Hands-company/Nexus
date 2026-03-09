-- Phase 19: Ecosystem & Onboarding (v1.8)
-- ─────────────────────────────────────────────────────────────────────────────

-- ── 19-01: Import Jobs ──────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS import_jobs (
    id              TEXT PRIMARY KEY,
    server_id       TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_platform TEXT NOT NULL,              -- discord | slack | matrix
    status          TEXT NOT NULL DEFAULT 'pending', -- pending | processing | completed | failed
    total_items     INT  NOT NULL DEFAULT 0,
    imported_items  INT  NOT NULL DEFAULT 0,
    error_log       TEXT,
    metadata        JSONB NOT NULL DEFAULT '{}', -- platform-specific config/mappings
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_import_jobs_server ON import_jobs(server_id);
CREATE INDEX IF NOT EXISTS idx_import_jobs_user   ON import_jobs(user_id, created_at DESC);

-- ── 19-01: Bulk Invitations ─────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS bulk_invitations (
    id          TEXT PRIMARY KEY,
    server_id   TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    inviter_id  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emails      JSONB NOT NULL DEFAULT '[]',       -- ["a@b.com", ...]
    status      TEXT NOT NULL DEFAULT 'pending',   -- pending | sending | completed | failed
    sent_count  INT  NOT NULL DEFAULT 0,
    total_count INT  NOT NULL DEFAULT 0,
    invite_code TEXT,                               -- reusable invite code for all recipients
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_bulk_inv_server ON bulk_invitations(server_id);

-- ── 19-02: Server Templates ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS server_templates (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    category    TEXT NOT NULL,                      -- gaming | education | teams | open-source | community
    icon_url    TEXT,
    channels    JSONB NOT NULL DEFAULT '[]',        -- [{name, type, topic?, position}]
    roles       JSONB NOT NULL DEFAULT '[]',        -- [{name, color, permissions}]
    settings    JSONB NOT NULL DEFAULT '{}',
    is_builtin  BOOLEAN NOT NULL DEFAULT FALSE,
    creator_id  TEXT REFERENCES users(id) ON DELETE SET NULL,
    usage_count INT  NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_templates_category ON server_templates(category);

-- ── 19-02: Onboarding Progress ─────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS onboarding_progress (
    user_id          TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    completed_steps  JSONB NOT NULL DEFAULT '[]',   -- ["create_server", "invite_friends", ...]
    dismissed        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── 19-03: Server Analytics Snapshots ───────────────────────────────────────
CREATE TABLE IF NOT EXISTS server_analytics_snapshots (
    id              TEXT PRIMARY KEY,
    server_id       TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    period_date     DATE NOT NULL,
    messages_count  INT  NOT NULL DEFAULT 0,
    active_members  INT  NOT NULL DEFAULT 0,
    new_members     INT  NOT NULL DEFAULT 0,
    left_members    INT  NOT NULL DEFAULT 0,
    voice_minutes   INT  NOT NULL DEFAULT 0,
    reports_resolved INT NOT NULL DEFAULT 0,
    bans_issued     INT  NOT NULL DEFAULT 0,
    filters_triggered INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(server_id, period_date)
);
CREATE INDEX IF NOT EXISTS idx_analytics_snap_server ON server_analytics_snapshots(server_id, period_date DESC);

-- ── 19-04: Plugin Marketplace ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS marketplace_plugins (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    slug            TEXT NOT NULL UNIQUE,
    description     TEXT,
    author_id       TEXT REFERENCES users(id) ON DELETE SET NULL,
    version         TEXT NOT NULL,
    manifest_url    TEXT NOT NULL,
    icon_url        TEXT,
    source_url      TEXT,                           -- git repo
    signature       TEXT,                            -- Ed25519 base64 signature
    signing_key_id  TEXT,                            -- which key signed this
    category        TEXT NOT NULL DEFAULT 'general', -- general | moderation | fun | utility | integration
    tags            JSONB NOT NULL DEFAULT '[]',
    downloads       BIGINT NOT NULL DEFAULT 0,
    avg_rating      REAL NOT NULL DEFAULT 0.0,
    rating_count    INT  NOT NULL DEFAULT 0,
    is_verified     BOOLEAN NOT NULL DEFAULT FALSE,
    is_published    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_mp_category  ON marketplace_plugins(category);
CREATE INDEX IF NOT EXISTS idx_mp_slug      ON marketplace_plugins(slug);
CREATE INDEX IF NOT EXISTS idx_mp_author    ON marketplace_plugins(author_id);
CREATE INDEX IF NOT EXISTS idx_mp_downloads ON marketplace_plugins(downloads DESC);

CREATE TABLE IF NOT EXISTS plugin_reviews (
    id          TEXT PRIMARY KEY,
    plugin_id   TEXT NOT NULL REFERENCES marketplace_plugins(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating      SMALLINT NOT NULL CHECK (rating >= 1 AND rating <= 5),
    title       TEXT,
    body        TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(plugin_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_pr_plugin ON plugin_reviews(plugin_id, created_at DESC);

CREATE TABLE IF NOT EXISTS plugin_installs (
    id          TEXT PRIMARY KEY,
    plugin_id   TEXT NOT NULL REFERENCES marketplace_plugins(id) ON DELETE CASCADE,
    server_id   TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    installed_by TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    version     TEXT NOT NULL,
    is_enabled  BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(plugin_id, server_id)
);
CREATE INDEX IF NOT EXISTS idx_pi_server ON plugin_installs(server_id);
