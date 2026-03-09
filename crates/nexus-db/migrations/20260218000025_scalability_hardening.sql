-- Phase 20: Scalability & Performance Hardening (v1.9)
-- Sub-phases: 20-01 Horizontal Scaling, 20-02 Federation Performance,
--             20-03 Voice Network Resilience, 20-04 Large-Server Tooling,
--             20-05 Operational Excellence

-- ── 20-01: Horizontal Scaling ─────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS scaling_configs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_id     TEXT NOT NULL UNIQUE,
    region          TEXT NOT NULL DEFAULT 'default',
    shard_strategy  TEXT NOT NULL DEFAULT 'hash',       -- hash | range | none
    redis_mode      TEXT NOT NULL DEFAULT 'standalone', -- standalone | sentinel | cluster
    gateway_weight  INT  NOT NULL DEFAULT 100,          -- load-balancer weight 1-1000
    max_connections INT  NOT NULL DEFAULT 10000,
    metadata        JSONB NOT NULL DEFAULT '{}',
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS sfu_nodes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_id     TEXT NOT NULL UNIQUE,
    region          TEXT NOT NULL,
    hostname        TEXT NOT NULL,
    port            INT  NOT NULL DEFAULT 3478,
    capacity        INT  NOT NULL DEFAULT 200,
    current_load    INT  NOT NULL DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'active', -- active | draining | offline
    metadata        JSONB NOT NULL DEFAULT '{}',
    last_heartbeat  TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── 20-02: Federation Performance ─────────────────────────────────────────

CREATE TABLE IF NOT EXISTS federation_event_batches (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_instance TEXT NOT NULL,
    events          JSONB NOT NULL DEFAULT '[]',
    event_count     INT  NOT NULL DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'pending', -- pending | sending | delivered | failed
    retry_count     INT  NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at         TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_fed_batches_status ON federation_event_batches(status);

CREATE TABLE IF NOT EXISTS federation_routes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_instance TEXT NOT NULL,
    target_instance TEXT NOT NULL,
    latency_ms      INT  NOT NULL DEFAULT 0,
    is_websocket    BOOLEAN NOT NULL DEFAULT FALSE,
    priority        INT  NOT NULL DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'active', -- active | degraded | offline
    last_probed     TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(source_instance, target_instance)
);

CREATE TABLE IF NOT EXISTS federation_dedup_log (
    event_id        TEXT NOT NULL,
    source_instance TEXT NOT NULL,
    received_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (event_id, source_instance)
);

-- ── 20-03: Voice Network Resilience ───────────────────────────────────────

CREATE TABLE IF NOT EXISTS voice_quality_logs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id      UUID NOT NULL,
    user_id         UUID NOT NULL,
    sfu_node_id     UUID REFERENCES sfu_nodes(id),
    bitrate         INT  NOT NULL DEFAULT 0,
    packet_loss     REAL NOT NULL DEFAULT 0.0,
    jitter_ms       REAL NOT NULL DEFAULT 0.0,
    latency_ms      INT  NOT NULL DEFAULT 0,
    fec_enabled     BOOLEAN NOT NULL DEFAULT FALSE,
    quality_score   REAL NOT NULL DEFAULT 1.0,  -- 0.0 (bad) .. 1.0 (perfect)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_voice_quality_channel ON voice_quality_logs(channel_id, created_at DESC);

-- ── 20-04: Large-Server Tooling ───────────────────────────────────────────

CREATE TABLE IF NOT EXISTS member_prune_rules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL,
    inactivity_days INT  NOT NULL DEFAULT 30,
    grace_period_days INT NOT NULL DEFAULT 7,
    exclude_roles   JSONB NOT NULL DEFAULT '[]',
    notify_before   BOOLEAN NOT NULL DEFAULT TRUE,
    is_enabled      BOOLEAN NOT NULL DEFAULT FALSE,
    last_run_at     TIMESTAMPTZ,
    pruned_count    INT  NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(server_id)
);

CREATE TABLE IF NOT EXISTS slow_mode_overrides (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id      UUID NOT NULL,
    role_id         UUID,           -- NULL = default for all roles
    cooldown_secs   INT  NOT NULL DEFAULT 5,
    escalation_mult REAL NOT NULL DEFAULT 1.0, -- multiplier per repeated violation
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(channel_id, role_id)
);

-- ── 20-05: Operational Excellence ─────────────────────────────────────────

CREATE TABLE IF NOT EXISTS scaling_metrics (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_id     TEXT NOT NULL,
    metric_name     TEXT NOT NULL,
    metric_value    REAL NOT NULL DEFAULT 0.0,
    unit            TEXT NOT NULL DEFAULT '',
    tags            JSONB NOT NULL DEFAULT '{}',
    recorded_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_scaling_metrics_instance ON scaling_metrics(instance_id, recorded_at DESC);

CREATE TABLE IF NOT EXISTS upgrade_records (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_version    TEXT NOT NULL,
    to_version      TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending', -- pending | rolling | completed | rolled_back
    started_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at    TIMESTAMPTZ,
    notes           TEXT,
    metadata        JSONB NOT NULL DEFAULT '{}'
);
