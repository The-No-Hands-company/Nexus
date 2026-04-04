-- Migration: add email_hash column for at-rest email encryption lookup
--
-- When NEXUS__SECURITY__EMAIL_KEY is set, email addresses are stored as
-- AES-256-GCM ciphertext in the existing `email` column (prefixed with
-- "enc:").  The plaintext `email` column is retained for backward
-- compatibility and for deployments that do not enable encryption.
--
-- `email_hash` stores an HMAC-SHA256 digest of the lowercased email so
-- that find_by_email lookups work without a full-table decrypt scan.
-- The column is NULL for rows created before encryption was enabled.

ALTER TABLE users ADD COLUMN IF NOT EXISTS email_hash TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_hash
    ON users (email_hash)
    WHERE email_hash IS NOT NULL;
