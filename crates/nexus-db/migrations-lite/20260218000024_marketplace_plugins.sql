-- Marketplace plugin registry — SQLite counterpart of the table created in
-- migrations/20260218000024_ecosystem_onboarding.sql.
--
-- The consolidated lite schema (20260218000001_initial.sql) claims to cover all
-- features but never created this one, so `nexus serve --lite` failed on
-- 20260402000001_store_governance with "no such table: marketplace_plugins" —
-- that migration rebuilds this table to add the governance columns, and there
-- was nothing to rebuild.
--
-- Numbered to match the Postgres migration that introduces it, so it lands
-- before the governance rewrite. Kept as its own file rather than folded into
-- the initial schema, which has already been applied to existing lite
-- databases and whose checksum must not change.
--
-- Column order matches what store_governance's INSERT ... SELECT expects.
-- Lite conventions: TEXT for UUIDs/timestamps/JSON, INTEGER for booleans.

CREATE TABLE IF NOT EXISTS marketplace_plugins (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    slug            TEXT NOT NULL UNIQUE,
    description     TEXT,
    author_id       TEXT REFERENCES users(id) ON DELETE SET NULL,
    version         TEXT NOT NULL,
    manifest_url    TEXT NOT NULL,
    icon_url        TEXT,
    source_url      TEXT,                            -- git repo
    signature       TEXT,                            -- Ed25519 base64 signature
    signing_key_id  TEXT,                            -- which key signed this
    category        TEXT NOT NULL DEFAULT 'general', -- general | moderation | fun | utility | integration
    tags            TEXT NOT NULL DEFAULT '[]',      -- JSON array
    downloads       INTEGER NOT NULL DEFAULT 0,
    avg_rating      REAL NOT NULL DEFAULT 0.0,
    rating_count    INTEGER NOT NULL DEFAULT 0,
    is_verified     INTEGER NOT NULL DEFAULT 0,
    is_published    INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_marketplace_plugins_slug ON marketplace_plugins(slug);
CREATE INDEX IF NOT EXISTS idx_marketplace_plugins_author ON marketplace_plugins(author_id);
