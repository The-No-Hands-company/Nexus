-- ============================================================================
-- Security: Instance-Level Audit Log
-- ============================================================================
-- For system-wide administrative actions (user suspension, admin grants, etc.)
-- that are not tied to a specific server.

CREATE TABLE IF NOT EXISTS instance_audit_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_id        UUID NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    action          VARCHAR(64) NOT NULL,          -- USER_SUSPEND, USER_UNSUSPEND, USER_DISABLE, ADMIN_GRANT, etc.
    target_type     VARCHAR(32),                   -- user, server, etc.
    target_id       UUID,                          -- UUID of affected entity
    changes         JSONB,                         -- Before/after values
    reason          TEXT,                          -- Human-provided reason
    ip_address      INET,                          -- Actor IP for accountability
    user_agent      TEXT,                          -- Client identifier
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_instance_audit_actor ON instance_audit_log (actor_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_instance_audit_target ON instance_audit_log (target_type, target_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_instance_audit_action ON instance_audit_log (action, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_instance_audit_time ON instance_audit_log (created_at DESC);
