-- Migration: v0.16 Server Discovery & Creator Monetization
-- Phase 15-04 (Creator Monetization) and 15-05 (Server Discovery Improvements)

-- ============================================================================
-- 15-05: Server Discovery — Tags, Categories, Popularity, Featured
-- ============================================================================

-- Tags (freeform, up to 5 per server) and category (single predefined)
ALTER TABLE servers
    ADD COLUMN IF NOT EXISTS tags         TEXT[]      NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS category     TEXT,
    ADD COLUMN IF NOT EXISTS activity_score INTEGER   NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS featured_at  TIMESTAMPTZ;

-- Fast lookup for tag-based discovery (GIN index on the array)
CREATE INDEX IF NOT EXISTS idx_servers_tags ON servers USING GIN (tags)
    WHERE is_public = true;

-- Category + popularity ordering
CREATE INDEX IF NOT EXISTS idx_servers_category ON servers (category, activity_score DESC)
    WHERE is_public = true AND category IS NOT NULL;

-- Featured servers (non-null featured_at, ordered by recency)
CREATE INDEX IF NOT EXISTS idx_servers_featured ON servers (featured_at DESC)
    WHERE is_public = true AND featured_at IS NOT NULL;

-- Full-text search on server name + description
CREATE INDEX IF NOT EXISTS idx_servers_search ON servers
    USING GIN (to_tsvector('english', COALESCE(name, '') || ' ' || COALESCE(description, '')))
    WHERE is_public = true;

-- ============================================================================
-- 15-04: Creator Monetization — Tip Jar, Subscriptions, Payments, Analytics
-- ============================================================================

-- Tip jar URL on servers (external payment link e.g. Ko-fi, Buy Me a Coffee)
ALTER TABLE servers
    ADD COLUMN IF NOT EXISTS tip_jar_url  TEXT;

-- Subscription-gated channels
ALTER TABLE channels
    ADD COLUMN IF NOT EXISTS subscription_required BOOLEAN NOT NULL DEFAULT FALSE;

-- Creator payment configuration per server
CREATE TABLE IF NOT EXISTS creator_payment_config (
    server_id       UUID        PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    -- Payment provider: 'stripe' | 'paypal' | 'custom_webhook' | 'none'
    provider        TEXT        NOT NULL DEFAULT 'none',
    -- Encrypted provider config (API keys, webhook URLs, etc.)
    provider_config JSONB       NOT NULL DEFAULT '{}',
    -- Webhook URL for payment notifications (self-hosted payment processing)
    webhook_url     TEXT,
    -- Webhook signing secret (for verifying incoming webhooks)
    webhook_secret  TEXT,
    -- Whether monetization is enabled for this server
    enabled         BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Subscription tiers defined by server owners
CREATE TABLE IF NOT EXISTS creator_subscription_tiers (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID        NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    description     TEXT        CHECK (char_length(description) <= 500),
    -- Price in smallest currency unit (cents)
    price_cents     INTEGER     NOT NULL CHECK (price_cents >= 0),
    currency        TEXT        NOT NULL DEFAULT 'USD' CHECK (char_length(currency) = 3),
    -- Billing interval: 'monthly' | 'yearly'
    interval        TEXT        NOT NULL DEFAULT 'monthly' CHECK (interval IN ('monthly', 'yearly')),
    -- Role granted to subscribers of this tier
    role_id         UUID        REFERENCES roles(id) ON DELETE SET NULL,
    -- Ordering position
    position        INTEGER     NOT NULL DEFAULT 0,
    is_active       BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_creator_sub_tiers_server ON creator_subscription_tiers (server_id, position);

-- User subscriptions
CREATE TABLE IF NOT EXISTS creator_subscriptions (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id       UUID        NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    tier_id         UUID        NOT NULL REFERENCES creator_subscription_tiers(id) ON DELETE CASCADE,
    -- Status: 'active' | 'cancelled' | 'past_due' | 'expired'
    status          TEXT        NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'cancelled', 'past_due', 'expired')),
    -- External subscription ID from payment provider
    external_id     TEXT,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ,
    cancelled_at    TIMESTAMPTZ,
    UNIQUE (user_id, server_id, tier_id)
);

CREATE INDEX IF NOT EXISTS idx_creator_subs_user   ON creator_subscriptions (user_id);
CREATE INDEX IF NOT EXISTS idx_creator_subs_server ON creator_subscriptions (server_id, status);

-- Creator analytics (daily aggregates, computed server-side)
CREATE TABLE IF NOT EXISTS creator_analytics (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID        NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    period_date     DATE        NOT NULL,
    -- Engagement metrics
    messages_count  INTEGER     NOT NULL DEFAULT 0,
    active_members  INTEGER     NOT NULL DEFAULT 0,
    new_members     INTEGER     NOT NULL DEFAULT 0,
    left_members    INTEGER     NOT NULL DEFAULT 0,
    voice_minutes   INTEGER     NOT NULL DEFAULT 0,
    -- Monetization metrics
    revenue_cents   INTEGER     NOT NULL DEFAULT 0,
    new_subscribers INTEGER     NOT NULL DEFAULT 0,
    churned_subscribers INTEGER NOT NULL DEFAULT 0,
    tip_count       INTEGER     NOT NULL DEFAULT 0,
    tip_total_cents INTEGER     NOT NULL DEFAULT 0,
    UNIQUE (server_id, period_date)
);

CREATE INDEX IF NOT EXISTS idx_creator_analytics_server ON creator_analytics (server_id, period_date DESC);
