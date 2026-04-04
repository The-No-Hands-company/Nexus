-- Migration: push_subscriptions — Web Push (RFC 8030 / VAPID) subscriber store
--
-- Each row represents one browser/client push subscription for a user.
-- A single user can have multiple subscriptions (e.g. multiple browsers or
-- devices). The endpoint, p256dh, and auth fields come directly from the
-- browser's PushSubscription object.

CREATE TABLE IF NOT EXISTS push_subscriptions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- The push service endpoint URL (unique per subscription)
    endpoint    TEXT NOT NULL UNIQUE,
    -- Client public key (base64url-encoded P-256 DH key)
    p256dh      TEXT NOT NULL,
    -- Client authentication secret (base64url-encoded 16-byte random)
    auth        TEXT NOT NULL,
    -- Optional UA string for display ("Chrome on Android", "Firefox on Desktop")
    user_agent  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Last time a push was successfully delivered to this endpoint
    last_used_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_push_subscriptions_user_id
    ON push_subscriptions (user_id);
