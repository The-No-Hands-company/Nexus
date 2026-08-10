-- Phantom Protocol — post-quantum identity keys (Kyber-1024 + Dilithium-5).
-- SQLite counterpart of migrations/20260810000001_phantom_identities.sql.
--
-- Lite conventions: TEXT for UUIDs and timestamps. Key material is base64 TEXT
-- in both backends, because the sqlx `Any` driver has no BYTEA codec.

CREATE TABLE IF NOT EXISTS phantom_identities (
    user_id         TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    did             TEXT NOT NULL UNIQUE,
    kem_public      TEXT NOT NULL,
    kem_secret      TEXT,
    signing_public  TEXT NOT NULL,
    signing_secret  TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS phantom_message_sigs (
    message_id  TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    phantom_did TEXT NOT NULL,
    signature   TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_phantom_message_sigs_user ON phantom_message_sigs (user_id, created_at DESC);
