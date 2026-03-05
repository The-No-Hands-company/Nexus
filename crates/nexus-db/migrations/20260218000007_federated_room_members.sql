-- Migration: Federated room membership tracking
--
-- Tracks which local users have joined federated rooms so that inbound
-- PDUs (messages) from remote servers can be dispatched via the gateway
-- to the right connected clients in real time.

CREATE TABLE IF NOT EXISTS federated_room_members (
    id        UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Fully-qualified room ID: !channel:server.tld
    room_id   TEXT        NOT NULL,
    -- The local user who joined this room.
    user_id   UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (room_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_fed_room_members_room ON federated_room_members (room_id);
CREATE INDEX IF NOT EXISTS idx_fed_room_members_user ON federated_room_members (user_id);
