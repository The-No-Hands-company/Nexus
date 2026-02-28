-- Migration: v0.8.5 Federation UX
-- Extends the federation layer with:
--   • Rich server identity (display name, description, admin contact, policy)
--   • Peer health tracking (latency, last error, online status)
--   • Peering request workflow (outbound / inbound invite flow)
--   • Trust scoring on peer connections

-- ─── Instance settings ───────────────────────────────────────────────────────
-- Generic key-value store for instance-level configuration.
-- Used by Phase 8.5 to store the instance's federation identity, but designed
-- to be reusable for future settings.

CREATE TABLE IF NOT EXISTS instance_settings (
    key         TEXT        PRIMARY KEY,
    value       JSONB       NOT NULL DEFAULT '{}',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ─── Enhance federated_servers ───────────────────────────────────────────────
-- Add health & metadata columns to the existing table.

ALTER TABLE federated_servers
    -- Human-readable display name fetched from remote's /.well-known
    ADD COLUMN IF NOT EXISTS display_name      TEXT,
    -- Short description from remote's /.well-known
    ADD COLUMN IF NOT EXISTS description       TEXT,
    -- Admin contact e-mail or URL (published at /.well-known)
    ADD COLUMN IF NOT EXISTS admin_contact     TEXT,
    -- Approximate active user count, fetched during connection or ping
    ADD COLUMN IF NOT EXISTS user_count        INTEGER,
    -- federation_policy: 'open' | 'allowlist' | 'closed'
    ADD COLUMN IF NOT EXISTS federation_policy TEXT NOT NULL DEFAULT 'open',
    -- Trust score 0-100 (0 = blocked, 25 = untrusted, 50 = default, 100 = verified)
    ADD COLUMN IF NOT EXISTS trust_score       SMALLINT NOT NULL DEFAULT 50
        CHECK (trust_score BETWEEN 0 AND 100),
    -- Most-recent measured round-trip latency in milliseconds
    ADD COLUMN IF NOT EXISTS latency_ms        INTEGER,
    -- Last error string from a failed ping / transaction
    ADD COLUMN IF NOT EXISTS last_error        TEXT,
    -- Whether the most recent health check succeeded
    ADD COLUMN IF NOT EXISTS is_healthy        BOOLEAN NOT NULL DEFAULT TRUE,
    -- Version string reported by the remote (e.g. "Nexus 0.15.0")
    ADD COLUMN IF NOT EXISTS software_version  TEXT;

-- Index for dashboard queries (order by trust, filter by health)
CREATE INDEX IF NOT EXISTS idx_federated_servers_trust
    ON federated_servers (trust_score DESC, is_healthy);

-- ─── Peering requests ─────────────────────────────────────────────────────────
-- Tracks outbound peering invites (we reached out) and inbound ones (they did).
-- An admin must accept an inbound request before that server is added to
-- federated_servers as a trusted peer (or the row already exists with low trust).

CREATE TABLE IF NOT EXISTS federation_peer_requests (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Remote server domain (e.g. "nexus.other.tld")
    remote_domain   TEXT        NOT NULL,
    -- 'outbound' = we initiated; 'inbound' = they reached out
    direction       TEXT        NOT NULL CHECK (direction IN ('outbound', 'inbound')),
    -- 'pending' | 'accepted' | 'rejected' | 'cancelled'
    status          TEXT        NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'accepted', 'rejected', 'cancelled')),
    -- Local admin who initiated or acted on the request
    acted_by        UUID        REFERENCES users(id) ON DELETE SET NULL,
    -- Optional human message included in the invitation
    message         TEXT,
    -- Peer's display name as discovered via /.well-known at request time
    remote_display_name TEXT,
    -- Peer's description as discovered via /.well-known at request time
    remote_description  TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_peer_requests_domain  ON federation_peer_requests (remote_domain);
CREATE INDEX IF NOT EXISTS idx_peer_requests_status  ON federation_peer_requests (status);
CREATE INDEX IF NOT EXISTS idx_peer_requests_dir     ON federation_peer_requests (direction, status);

-- ─── Federation audit log ─────────────────────────────────────────────────────
-- Append-only log of admin actions on the federation layer so instance admins
-- can audit who changed trust levels, accepted peers, blocked servers, etc.

CREATE TABLE IF NOT EXISTS federation_audit_log (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The admin user who performed the action (NULL = system / health sweep)
    admin_id        UUID        REFERENCES users(id) ON DELETE SET NULL,
    -- Action type: 'peer_added' | 'peer_removed' | 'trust_changed' | 'peer_blocked' |
    --              'request_accepted' | 'request_rejected' | 'health_sweep'
    action          TEXT        NOT NULL,
    -- Domain the action was performed on
    target_domain   TEXT        NOT NULL,
    -- JSON payload with extra context (old/new trust_score, etc.)
    details         JSONB       NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_fed_audit_domain ON federation_audit_log (target_domain, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_fed_audit_admin  ON federation_audit_log (admin_id, created_at DESC);
