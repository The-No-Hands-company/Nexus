-- Store Governance & Trust Model — Phase 20-01 (SQLite Lite)
-- Simplified version for single-binary SQLite deployment

-- Review status values (SQLite doesn't have ENUM, use TEXT with CHECK)
CREATE TABLE IF NOT EXISTS marketplace_plugins_updated (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    author_id TEXT REFERENCES users(id),
    version TEXT NOT NULL,
    manifest_url TEXT NOT NULL,
    icon_url TEXT,
    source_url TEXT,
    signature TEXT,
    signing_key_id TEXT,
    category TEXT NOT NULL,
    tags TEXT, -- JSON
    downloads INTEGER DEFAULT 0,
    avg_rating REAL DEFAULT 0,
    rating_count INTEGER DEFAULT 0,
    is_verified BOOLEAN DEFAULT 0,
    is_published BOOLEAN DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Governance fields
    review_status TEXT DEFAULT 'draft' CHECK(review_status IN ('draft','submitted','scanning','review','approved','rejected','quarantined','takedown','archived')),
    trust_tier TEXT DEFAULT 'unlisted' CHECK(trust_tier IN ('unlisted','reviewed','verified')),
    creator_identity_level TEXT DEFAULT 'unverified' CHECK(creator_identity_level IN ('unverified','email_verified','domain_verified','signature_verified')),
    provenance_verified BOOLEAN DEFAULT 0,
    rights_attestation TEXT,
    security_scan_result TEXT, -- JSON
    last_scanned_at DATETIME,
    rejected_reason TEXT,
    quarantine_reason TEXT,
    review_notes TEXT,
    reviewer_id TEXT REFERENCES users(id),
    reviewed_at DATETIME,
    takedown_requested_by TEXT REFERENCES users(id),
    takedown_reason TEXT,
    takedown_requested_at DATETIME
);

-- Copy existing data
INSERT INTO marketplace_plugins_updated 
SELECT id, name, slug, description, author_id, version, manifest_url, icon_url, source_url, 
       signature, signing_key_id, category, tags, downloads, avg_rating, rating_count, 
       is_verified, is_published, created_at, updated_at,
       'draft', 'unlisted', 'unverified', 0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL
FROM marketplace_plugins;

-- Drop old table and rename new
DROP TABLE marketplace_plugins;
ALTER TABLE marketplace_plugins_updated RENAME TO marketplace_plugins;

-- Create indices
CREATE INDEX idx_marketplace_plugins_discovery ON marketplace_plugins(review_status, trust_tier, downloads) WHERE is_published = 1;
CREATE INDEX idx_marketplace_plugins_review_queue ON marketplace_plugins(review_status, created_at DESC) WHERE review_status IN ('submitted','scanning','review');

-- Review audit trail
CREATE TABLE IF NOT EXISTS marketplace_reviews (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL REFERENCES marketplace_plugins(id) ON DELETE CASCADE,
    reviewer_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    previous_status TEXT NOT NULL,
    new_status TEXT NOT NULL,
    action TEXT NOT NULL,
    reason TEXT,
    notes TEXT,
    metadata TEXT, -- JSON
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_marketplace_reviews_plugin ON marketplace_reviews(plugin_id, created_at DESC);

-- Creator vetting — privacy by design: no personal data stored
CREATE TABLE IF NOT EXISTS creator_vetting (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    identity_level TEXT NOT NULL DEFAULT 'unverified' CHECK(identity_level IN ('unverified','email_verified','domain_verified','signature_verified')),
    domain TEXT,
    domain_verified BOOLEAN DEFAULT 0,
    signing_key_fingerprint TEXT, -- Public key fingerprint only; creator publishes their own key
    signing_key_type TEXT,        -- 'pgp', 'ssh', or 'minisign'
    rights_attestation TEXT,
    two_factor_enabled BOOLEAN DEFAULT 0,
    ip_whitelist TEXT, -- JSON
    status TEXT DEFAULT 'pending' CHECK(status IN ('pending','approved','rejected','suspended')),
    approved_by TEXT REFERENCES users(id),
    approved_at DATETIME,
    rejection_reason TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_creator_vetting_user ON creator_vetting(user_id, status);

-- Monetization metadata — TNHC is payments-neutral; never processes or stores transactions
CREATE TABLE IF NOT EXISTS marketplace_monetization (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL UNIQUE REFERENCES marketplace_plugins(id) ON DELETE CASCADE,
    creator_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_monetized BOOLEAN DEFAULT 0,
    price_cents INTEGER,
    currency TEXT DEFAULT 'USD',
    payment_link TEXT,       -- Creator's external payment URL; TNHC redirects, never touches money
    purchase_count INTEGER DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_marketplace_monetization_creator ON marketplace_monetization(creator_id, is_monetized);

-- Quarantine and takedown workflow
CREATE TABLE IF NOT EXISTS marketplace_takedowns (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL REFERENCES marketplace_plugins(id) ON DELETE CASCADE,
    reported_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    reason TEXT NOT NULL,
    description TEXT NOT NULL,
    evidence_urls TEXT, -- JSON
    quarantine_status TEXT DEFAULT 'pending' CHECK(quarantine_status IN ('pending','quarantined','reviewed','reinstated','permanent_takedown')),
    reviewer_id TEXT REFERENCES users(id),
    review_notes TEXT,
    reviewed_at DATETIME,
    reinstated_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_marketplace_takedowns_status ON marketplace_takedowns(quarantine_status, created_at DESC);
