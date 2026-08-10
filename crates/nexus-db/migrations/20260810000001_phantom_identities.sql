-- Phantom Protocol — post-quantum identity keys (Kyber-1024 + Dilithium-5).
--
-- The API surface for this (POST /api/users/@me/phantom, GET
-- /api/users/{id}/phantom) has been mounted since the feature landed, but the
-- tables it reads only ever existed in an ad-hoc repository::phantom::
-- run_migration() that nothing called — so every request against it failed.
-- The schema belongs here with the rest of the tables.
--
-- Key material is TEXT (base64), not BYTEA: the sqlx `Any` driver used for
-- Postgres/SQLite portability has no BYTEA codec. The public keys were already
-- base64 for that reason; the secrets now match.

CREATE TABLE IF NOT EXISTS phantom_identities (
    user_id         UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    did             TEXT NOT NULL UNIQUE,           -- did:phantom:<blake3 prefix>
    kem_public      TEXT NOT NULL,                  -- base64 Kyber-1024 public key
    kem_secret      TEXT,                           -- base64; encrypt at rest in production
    signing_public  TEXT NOT NULL,                  -- base64 Dilithium-5 public key
    signing_secret  TEXT,                           -- base64; encrypt at rest in production
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS phantom_message_sigs (
    message_id  UUID PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    phantom_did TEXT NOT NULL,
    signature   TEXT NOT NULL,                      -- base64 Dilithium-5 detached signature
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_phantom_message_sigs_user ON phantom_message_sigs (user_id, created_at DESC);
