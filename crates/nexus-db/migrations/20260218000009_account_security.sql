-- ============================================================
-- 09.7 Account Security: TOTP 2FA, Email Verification, Sessions
-- ============================================================

-- ── TOTP / Backup codes columns on users ────────────────────
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS totp_secret          TEXT,          -- base32-encoded TOTP secret (AES-256 encrypted at rest)
    ADD COLUMN IF NOT EXISTS totp_enabled         BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS scheduled_deletion_at TIMESTAMPTZ;  -- set when user requests deletion; purged after 30 days

-- ── TOTP backup codes ────────────────────────────────────────
-- 8 single-use backup codes per user.
-- code_hash stores SHA-256(raw_code) to avoid storing plaintext.
CREATE TABLE IF NOT EXISTS totp_backup_codes (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash  TEXT        NOT NULL,           -- hex(SHA-256(raw_code))
    used_at    TIMESTAMPTZ,                    -- NULL = not yet consumed
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_totp_backup_user ON totp_backup_codes(user_id);

-- ── Email verification tokens ────────────────────────────────
CREATE TABLE IF NOT EXISTS email_verification_tokens (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT        NOT NULL UNIQUE,    -- hex(SHA-256(raw_token))
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_email_verification_user
    ON email_verification_tokens(user_id);

-- ── Extend refresh_tokens for proper session management ───────
-- refresh_tokens was created in the initial schema but never written to.
-- We extend it here and start persisting tokens in 09.7-03.
ALTER TABLE refresh_tokens
    ADD COLUMN IF NOT EXISTS user_agent   TEXT,
    ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
