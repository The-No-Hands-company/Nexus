-- Phase 24: Long-Term Sustainability & Extensibility (v2.x)
-- Sub-phases: 24-01 Protocol Evolution, 24-02 Community Governance,
--             24-03 Security & Threat Modeling, 24-04 Documentation & Education

-- ── 24-01: Protocol Evolution ─────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS protocol_versions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol        TEXT NOT NULL,           -- federation | plugin | bot_api
    version         TEXT NOT NULL,           -- semver: 1.0.0, 2.0.0
    status          TEXT NOT NULL DEFAULT 'active', -- active | deprecated | sunset
    capabilities    JSONB NOT NULL DEFAULT '[]',
    deprecation_date TIMESTAMPTZ,
    sunset_date     TIMESTAMPTZ,
    migration_guide TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(protocol, version)
);

CREATE TABLE IF NOT EXISTS capability_negotiations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    local_instance  TEXT NOT NULL,
    remote_instance TEXT NOT NULL,
    local_caps      JSONB NOT NULL DEFAULT '[]',
    remote_caps     JSONB NOT NULL DEFAULT '[]',
    agreed_caps     JSONB NOT NULL DEFAULT '[]',
    protocol_version TEXT NOT NULL,
    negotiated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(local_instance, remote_instance)
);

-- ── 24-02: Community Governance ───────────────────────────────────────────

CREATE TABLE IF NOT EXISTS governance_polls (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL,
    title           TEXT NOT NULL,
    description     TEXT,
    poll_type       TEXT NOT NULL DEFAULT 'advisory', -- advisory | binding
    options         JSONB NOT NULL DEFAULT '[]',
    min_participation REAL NOT NULL DEFAULT 0.0,      -- fraction of members needed
    allow_multiple  BOOLEAN NOT NULL DEFAULT FALSE,
    anonymous       BOOLEAN NOT NULL DEFAULT TRUE,
    status          TEXT NOT NULL DEFAULT 'open',     -- open | closed | cancelled
    created_by      UUID NOT NULL,
    opens_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    closes_at       TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS poll_votes (
    poll_id         UUID NOT NULL REFERENCES governance_polls(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL,
    option_index    INT  NOT NULL,
    voted_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (poll_id, user_id, option_index)
);

CREATE TABLE IF NOT EXISTS governance_proposals (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL,
    title           TEXT NOT NULL,
    body            TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'draft', -- draft | discussion | voting | accepted | rejected
    author_id       UUID NOT NULL,
    discussion_channel UUID,
    poll_id         UUID REFERENCES governance_polls(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS contributor_badges (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    badge_type      TEXT NOT NULL,       -- code_contributor | bug_reporter | translator | donor
    source          TEXT,                -- github_username, etc.
    verified        BOOLEAN NOT NULL DEFAULT FALSE,
    metadata        JSONB NOT NULL DEFAULT '{}',
    awarded_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, badge_type)
);

-- ── 24-03: Security & Threat Modeling ─────────────────────────────────────

CREATE TABLE IF NOT EXISTS security_audits (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    audit_type      TEXT NOT NULL,       -- pq_readiness | dependency_scan | pen_test | manual
    status          TEXT NOT NULL DEFAULT 'scheduled', -- scheduled | in_progress | completed | failed
    findings        JSONB NOT NULL DEFAULT '[]',
    severity_summary JSONB NOT NULL DEFAULT '{}',      -- { critical: 0, high: 1, medium: 3, low: 5 }
    auditor         TEXT,
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS vulnerability_records (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    audit_id        UUID REFERENCES security_audits(id),
    cve_id          TEXT,
    package_name    TEXT NOT NULL,
    severity        TEXT NOT NULL DEFAULT 'medium', -- critical | high | medium | low | info
    description     TEXT NOT NULL,
    remediation     TEXT,
    status          TEXT NOT NULL DEFAULT 'open',   -- open | mitigated | resolved | accepted
    discovered_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at     TIMESTAMPTZ
);

-- ── 24-04: Documentation & Education ──────────────────────────────────────

CREATE TABLE IF NOT EXISTS tutorial_progress (
    user_id         UUID NOT NULL,
    tutorial_id     TEXT NOT NULL,        -- self_host_setup | federation_peering | plugin_dev | first_pr
    completed_steps JSONB NOT NULL DEFAULT '[]',
    completed       BOOLEAN NOT NULL DEFAULT FALSE,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at    TIMESTAMPTZ,
    PRIMARY KEY (user_id, tutorial_id)
);

CREATE TABLE IF NOT EXISTS migration_guides (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_platform   TEXT NOT NULL,        -- discord | slack | matrix | teams | telegram
    title           TEXT NOT NULL,
    content         TEXT NOT NULL,
    version         TEXT NOT NULL DEFAULT '1.0',
    is_published    BOOLEAN NOT NULL DEFAULT FALSE,
    author_id       UUID,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(from_platform, version)
);
