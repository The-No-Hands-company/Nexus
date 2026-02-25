-- Migration: Matrix Application Service Bridge
-- Adds persistence for channel ↔ Matrix room mappings and ghost user tracking.

-- ─── Channel ↔ Matrix room mapping ────────────────────────────────────────────

-- Maps a Nexus channel to one Matrix room (one-to-one relationship).
-- When a message is created in a bridged channel it is relayed to the Matrix
-- room, and inbound Matrix events for that room are dispatched to the channel.
CREATE TABLE IF NOT EXISTS matrix_bridge_rooms (
    channel_id      UUID        NOT NULL,   -- Nexus channel UUID
    matrix_room_id  TEXT        NOT NULL,   -- Matrix room ID (!id:server)
    homeserver_url  TEXT        NOT NULL,   -- Matrix HS base URL for outbound relay
    as_token        TEXT        NOT NULL DEFAULT '',  -- Application service token
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (channel_id),
    UNIQUE (matrix_room_id)
);

CREATE INDEX idx_matrix_bridge_rooms_room ON matrix_bridge_rooms (matrix_room_id);

-- ─── Ghost / puppet users ─────────────────────────────────────────────────────

-- A ghost user is a synthetic Nexus account that represents a Matrix participant
-- inside Nexus.  One ghost is created per distinct MXID seen in any bridged room.
-- The ghost's UUID is stable so it can be referenced in messages, reactions, etc.
CREATE TABLE IF NOT EXISTS matrix_ghost_users (
    id          UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    mxid        TEXT    NOT NULL UNIQUE,     -- Full Matrix ID: @user:server
    username    TEXT    NOT NULL UNIQUE,     -- Synthetic nexus username: matrix_user_server
    display_name TEXT,
    avatar_url  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_matrix_ghost_users_mxid ON matrix_ghost_users (mxid);
