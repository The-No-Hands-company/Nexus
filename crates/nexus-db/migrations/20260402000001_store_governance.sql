-- Store Governance & Trust Model — Phase 20-01
-- Adds review state machine, creator identity verification, and trust tiers.

-- Review status state machine: draft → submitted → scanning → review → approved/rejected/quarantined
CREATE TYPE review_status AS ENUM (
    'draft',           -- Created but not submitted for review
    'submitted',       -- Submitted for review, awaiting initial scan
    'scanning',        -- Automated security/compliance scanning in progress
    'review',          -- Manual human review in progress
    'approved',        -- Approved for marketplace publication
    'rejected',        -- Rejected with reason
    'quarantined',     -- Dangerous or suspicious, removed from marketplace
    'takedown',        -- Copyright/abuse reported, marked for removal
    'archived'         -- Creator deleted, preserved for audit
);

-- Trust tier: represents marketplace confidence in creator + content
CREATE TYPE trust_tier AS ENUM (
    'unlisted',        -- No tier; hidden from search (sandbox testing)
    'reviewed',        -- Human-reviewed; basic trustworthiness verified
    'verified'         -- Creator identity verified + content passes strict scanning
);

-- Identity verification level (cryptographic, no personal data stored)
CREATE TYPE identity_level AS ENUM (
    'unverified',         -- Account created; no verification
    'email_verified',     -- Email ownership confirmed
    'domain_verified',    -- Organization domain verified via DNS TXT record
    'signature_verified'  -- Controls a public cryptographic key (PGP/SSH/minisign)
                          -- Key is publicly published; TNHC stores only the fingerprint
);

-- Extend marketplace_plugins with governance fields
ALTER TABLE marketplace_plugins
    ADD COLUMN IF NOT EXISTS review_status review_status DEFAULT 'draft',
    ADD COLUMN IF NOT EXISTS trust_tier trust_tier DEFAULT 'unlisted',
    ADD COLUMN IF NOT EXISTS creator_identity_level identity_level DEFAULT 'unverified',
    ADD COLUMN IF NOT EXISTS provenance_verified BOOLEAN DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS rights_attestation TEXT,
    ADD COLUMN IF NOT EXISTS security_scan_result JSONB,
    ADD COLUMN IF NOT EXISTS last_scanned_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS rejected_reason TEXT,
    ADD COLUMN IF NOT EXISTS quarantine_reason TEXT,
    ADD COLUMN IF NOT EXISTS review_notes TEXT,
    ADD COLUMN IF NOT EXISTS reviewer_id UUID REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS reviewed_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS takedown_requested_by UUID REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS takedown_reason TEXT,
    ADD COLUMN IF NOT EXISTS takedown_requested_at TIMESTAMPTZ;

-- Index for discovery: marketplace listing should filter by review_status and trust_tier
CREATE INDEX IF NOT EXISTS idx_marketplace_plugins_discovery 
    ON marketplace_plugins(review_status, trust_tier, downloads) 
    WHERE is_published = TRUE;

-- Index for admin review queue
CREATE INDEX IF NOT EXISTS idx_marketplace_plugins_review_queue
    ON marketplace_plugins(review_status, created_at DESC)
    WHERE review_status IN ('submitted', 'scanning', 'review');

-- Table for review audit trail
CREATE TABLE IF NOT EXISTS marketplace_reviews (
    id UUID PRIMARY KEY,
    plugin_id TEXT NOT NULL REFERENCES marketplace_plugins(id) ON DELETE CASCADE,
    reviewer_id UUID REFERENCES users(id) ON DELETE SET NULL,
    previous_status review_status NOT NULL,
    new_status review_status NOT NULL,
    action TEXT NOT NULL, -- 'approved', 'rejected', 'quarantined', 'takedown_initiated', etc.
    reason TEXT,
    notes TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_marketplace_reviews_plugin
    ON marketplace_reviews(plugin_id, created_at DESC);

-- Table for creator vetting workflow — privacy by design: no personal data stored
CREATE TABLE IF NOT EXISTS creator_vetting (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    identity_level identity_level NOT NULL DEFAULT 'unverified',
    domain TEXT,              -- Optional: organization domain for DNS verification
    domain_verified BOOLEAN DEFAULT FALSE, -- Set after TNHC confirms DNS TXT record
    signing_key_fingerprint TEXT, -- Public key fingerprint only (not the key itself)
                                   -- Creator publishes their own key; we just index it
    signing_key_type TEXT,    -- 'pgp', 'ssh', or 'minisign'
    rights_attestation TEXT,  -- Creator confirms they own rights to publish; boolean flag
    two_factor_enabled BOOLEAN DEFAULT FALSE,
    ip_whitelist JSONB,       -- Optional creator-opted IP restrictions; security feature
    status VARCHAR(50) DEFAULT 'pending', -- pending, approved, rejected, suspended
    approved_by UUID REFERENCES users(id) ON DELETE SET NULL,
    approved_at TIMESTAMPTZ,
    rejection_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_creator_vetting_user
    ON creator_vetting(user_id, status);

-- Table for monetization metadata — TNHC is payments-neutral: we never process
-- or store payment data. Creators set their own pricing and payment links.
-- TNHC facilitates download gating but never touches money.
CREATE TABLE IF NOT EXISTS marketplace_monetization (
    id UUID PRIMARY KEY,
    plugin_id TEXT NOT NULL UNIQUE REFERENCES marketplace_plugins(id) ON DELETE CASCADE,
    creator_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_monetized BOOLEAN DEFAULT FALSE,
    price_cents INTEGER,     -- NULL for free; >0 for paid (display only)
    currency VARCHAR(3) DEFAULT 'USD',
    payment_link TEXT,       -- Creator's external payment URL (e.g., Stripe, Ko-fi)
                             -- TNHC redirects here; never processes or stores transactions
    purchase_count BIGINT DEFAULT 0, -- Non-financial counter; incremented on each install
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_marketplace_monetization_creator
    ON marketplace_monetization(creator_id, is_monetized);

-- Quarantine and takedown workflow table
CREATE TABLE IF NOT EXISTS marketplace_takedowns (
    id UUID PRIMARY KEY,
    plugin_id TEXT NOT NULL REFERENCES marketplace_plugins(id) ON DELETE CASCADE,
    reported_by UUID REFERENCES users(id) ON DELETE SET NULL,
    reason VARCHAR(100) NOT NULL, -- 'copyright', 'malware', 'abuse', 'spam', 'tos_violation', etc.
    description TEXT NOT NULL,
    evidence_urls JSONB, -- Array of URLs pointing to evidence (if public)
    quarantine_status VARCHAR(50) DEFAULT 'pending', -- pending, quarantined, reviewed, reinstated, permanent_takedown
    reviewer_id UUID REFERENCES users(id) ON DELETE SET NULL,
    review_notes TEXT,
    reviewed_at TIMESTAMPTZ,
    reinstated_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_marketplace_takedowns_status
    ON marketplace_takedowns(quarantine_status, created_at DESC);
