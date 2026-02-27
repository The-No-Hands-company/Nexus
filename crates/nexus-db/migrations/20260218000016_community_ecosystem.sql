-- Nexus Database Schema
-- Migration: v0.15 Community Ecosystem — badges, supporter tiers, canvas channels
-- Generated for PostgreSQL 15+

-- ============================================================================
-- 15-01: User Badges
-- ============================================================================

CREATE TABLE IF NOT EXISTS user_badges (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    badge_type      TEXT        NOT NULL,   -- early_adopter | active_contributor | verified_developer | server_booster | custom
    -- For custom badges: the server that owns them (null for global badges)
    server_id       UUID        REFERENCES servers(id) ON DELETE CASCADE,
    -- Who awarded it (null for automatically-awarded badges)
    awarded_by      UUID        REFERENCES users(id) ON DELETE SET NULL,
    awarded_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Optional display metadata for custom badges
    label           TEXT,
    icon_url        TEXT,
    UNIQUE (user_id, badge_type, COALESCE(server_id, '00000000-0000-0000-0000-000000000000'::uuid))
);

CREATE INDEX IF NOT EXISTS idx_user_badges_user ON user_badges (user_id);
CREATE INDEX IF NOT EXISTS idx_user_badges_server ON user_badges (server_id) WHERE server_id IS NOT NULL;

-- ============================================================================
-- 15-02: Server Supporter Tiers
-- ============================================================================

-- Tier metadata per server (auto-maintained by the boost logic)
CREATE TABLE IF NOT EXISTS server_supporter_tiers (
    server_id       UUID        PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    tier_level      SMALLINT    NOT NULL DEFAULT 0 CHECK (tier_level BETWEEN 0 AND 3),
    booster_count   INT         NOT NULL DEFAULT 0,
    -- Thresholds: tier1 ≥ 2, tier2 ≥ 7, tier3 ≥ 14 (Discord-inspired)
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Individual boosts
CREATE TABLE IF NOT EXISTS server_boosters (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id   UUID        NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    -- Each user can hold 2 boost slots (mirroring Discord's model)
    slot        SMALLINT    NOT NULL DEFAULT 1 CHECK (slot IN (1, 2)),
    started_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ,           -- null = active until manually cancelled
    UNIQUE (user_id, server_id, slot)
);

CREATE INDEX IF NOT EXISTS idx_server_boosters_server  ON server_boosters (server_id, expires_at);
CREATE INDEX IF NOT EXISTS idx_server_boosters_user    ON server_boosters (user_id);

-- Add vanity_url column to servers for tier 2+ vanity invite codes
-- (vanity_code already exists on the servers table — vanity_url is the resolved
--  full URL stored for quick access)
ALTER TABLE servers
    ADD COLUMN IF NOT EXISTS boost_tier     SMALLINT    NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS booster_count  INT         NOT NULL DEFAULT 0;

-- Ensure vanity_code uniqueness index exists
CREATE UNIQUE INDEX IF NOT EXISTS idx_servers_vanity_code ON servers (vanity_code)
    WHERE vanity_code IS NOT NULL;

-- ============================================================================
-- 15-03: Canvas (Rich Document) Channels
-- ============================================================================

-- Add canvas channel type support — the CHECK constraint on channel_type is
-- permissive (text column) so no ALTER needed; we just need the blocks table.
CREATE TABLE IF NOT EXISTS canvas_blocks (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id      UUID        NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    -- block_type: heading | paragraph | image | code | divider | table | callout
    block_type      TEXT        NOT NULL DEFAULT 'paragraph',
    -- block content stored as JSONB to allow rich/typed payloads
    content         JSONB       NOT NULL DEFAULT '{}',
    -- absolute ordering position (gap of 1000 to allow easy reordering)
    position        INT         NOT NULL DEFAULT 0,
    updated_by      UUID        REFERENCES users(id) ON DELETE SET NULL,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_canvas_blocks_channel ON canvas_blocks (channel_id, position);
