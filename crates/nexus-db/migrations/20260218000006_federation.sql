-- Migration: v0.8 Federation
-- Tables for server-to-server federation, federated identities, and the
-- public server directory.

-- ─── Server signing keys ──────────────────────────────────────────────────────

-- Stores the Ed25519 signing key pairs used to authenticate federation requests.
-- A server typically has one active key at a time; old keys are retained for
-- verification of in-flight transactions during rotation.
CREATE TABLE IF NOT EXISTS federation_keys (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    key_id          TEXT        NOT NULL UNIQUE,   -- e.g. "ed25519:3f9a2c"
    seed_bytes      BYTEA       NOT NULL,           -- 32-byte Ed25519 seed (encrypted at rest)
    public_key_b64  TEXT        NOT NULL,           -- base64url public key
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL,           -- 90 days from creation
    is_active       BOOLEAN     NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_federation_keys_active ON federation_keys (is_active, expires_at);

-- ─── Known remote servers ─────────────────────────────────────────────────────

-- Registry of all remote Nexus (and Matrix) servers we have interacted with.
CREATE TABLE IF NOT EXISTS federated_servers (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    server_name     TEXT        NOT NULL UNIQUE,    -- e.g. "nexus.other.tld"
    server_version  TEXT,
    -- Cached public keys for signature verification (JSON map of key_id → base64 pubkey)
    verify_keys     JSONB       NOT NULL DEFAULT '{}',
    -- When the cached key document expires and must be re-fetched
    keys_valid_until TIMESTAMPTZ,
    first_seen_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Manual block: set to TRUE to reject all requests from this server
    is_blocked      BOOLEAN     NOT NULL DEFAULT FALSE,
    -- Category: 'nexus' | 'matrix' | 'discord_bridge'
    server_type     TEXT        NOT NULL DEFAULT 'nexus',
    -- Base URL override (set when .well-known delegation is used)
    base_url        TEXT
);

CREATE INDEX idx_federated_servers_blocked ON federated_servers (is_blocked);

-- ─── Federated users ─────────────────────────────────────────────────────────

-- Remote users (from other servers) who have joined channels on this server,
-- or whose profiles have been resolved.
CREATE TABLE IF NOT EXISTS federated_users (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Fully-qualified Matrix-style user ID: @user:server.tld
    mxid            TEXT        NOT NULL UNIQUE,
    -- Local username portion
    localpart       TEXT        NOT NULL,
    server_id       UUID        NOT NULL REFERENCES federated_servers(id) ON DELETE CASCADE,
    display_name    TEXT,
    avatar_url      TEXT,
    -- Raw profile data as last seen from the remote server
    profile_json    JSONB       NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_federated_users_server ON federated_users (server_id);
CREATE INDEX idx_federated_users_mxid   ON federated_users (mxid);

-- ─── Federated rooms ─────────────────────────────────────────────────────────

-- Channels / rooms shared across server boundaries.
CREATE TABLE IF NOT EXISTS federated_rooms (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Fully-qualified room ID: !channelid:server.tld
    room_id         TEXT        NOT NULL UNIQUE,
    -- If this room is owned by our server, this is the local channel_id
    local_channel_id UUID       REFERENCES channels(id) ON DELETE SET NULL,
    -- Originating server
    origin_server   TEXT        NOT NULL,
    room_name       TEXT,
    room_topic      TEXT,
    join_rule       TEXT        NOT NULL DEFAULT 'public',   -- 'public' | 'invite' | 'knock'
    member_count    INTEGER     NOT NULL DEFAULT 0,
    -- Servers that are currently participating in this room
    participating_servers TEXT[] NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_federated_rooms_origin    ON federated_rooms (origin_server);
CREATE INDEX idx_federated_rooms_channel   ON federated_rooms (local_channel_id);

-- ─── Federated events ────────────────────────────────────────────────────────

-- Persistent Data Units (PDUs) received from remote servers and stored
-- locally so they can be served to other participants.
CREATE TABLE IF NOT EXISTS federated_events (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id        TEXT        NOT NULL UNIQUE,    -- "$<base64>:server.tld"
    room_id         TEXT        NOT NULL,           -- "!channel:server.tld"
    event_type      TEXT        NOT NULL,           -- "nexus.message.create" etc.
    sender          TEXT        NOT NULL,           -- "@user:server.tld"
    origin_server   TEXT        NOT NULL,
    origin_server_ts BIGINT     NOT NULL,           -- Unix ms from origin
    content         JSONB       NOT NULL DEFAULT '{}',
    -- Ed25519 signatures as received
    signatures      JSONB       NOT NULL DEFAULT '{}',
    -- SHA-256 content hash for integrity
    content_hash    TEXT,
    -- Which transaction this arrived in
    txn_id          TEXT,
    received_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Soft-delete: redacted events keep their row but content is cleared
    is_redacted     BOOLEAN     NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_federated_events_room      ON federated_events (room_id, origin_server_ts DESC);
CREATE INDEX idx_federated_events_sender    ON federated_events (sender);
CREATE INDEX idx_federated_events_origin    ON federated_events (origin_server);

-- ─── Federation transactions log ─────────────────────────────────────────────

-- Audit log of processed inbound transactions (for idempotency checks).
CREATE TABLE IF NOT EXISTS federation_txn_log (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    txn_id          TEXT        NOT NULL,
    origin_server   TEXT        NOT NULL,
    pdu_count       INTEGER     NOT NULL DEFAULT 0,
    edu_count       INTEGER     NOT NULL DEFAULT 0,
    received_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (txn_id, origin_server)
);

-- ─── Public directory ────────────────────────────────────────────────────────

-- Servers that have opted into the public directory.
-- This powers `GET /api/v1/directory/servers`.
CREATE TABLE IF NOT EXISTS directory_servers (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    server_name     TEXT        NOT NULL UNIQUE,
    description     TEXT,
    icon_url        TEXT,
    -- Cached counts — refreshed periodically by a background job
    public_room_count INTEGER   NOT NULL DEFAULT 0,
    total_users     INTEGER     NOT NULL DEFAULT 0,
    listed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ─── Triggers ────────────────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.triggers
        WHERE trigger_name = 'federated_users_updated_at'
    ) THEN
        CREATE TRIGGER federated_users_updated_at
        BEFORE UPDATE ON federated_users
        FOR EACH ROW EXECUTE FUNCTION update_updated_at();
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM information_schema.triggers
        WHERE trigger_name = 'federated_rooms_updated_at'
    ) THEN
        CREATE TRIGGER federated_rooms_updated_at
        BEFORE UPDATE ON federated_rooms
        FOR EACH ROW EXECUTE FUNCTION update_updated_at();
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM information_schema.triggers
        WHERE trigger_name = 'directory_servers_updated_at'
    ) THEN
        CREATE TRIGGER directory_servers_updated_at
        BEFORE UPDATE ON directory_servers
        FOR EACH ROW EXECUTE FUNCTION update_updated_at();
    END IF;
END;
$$;
