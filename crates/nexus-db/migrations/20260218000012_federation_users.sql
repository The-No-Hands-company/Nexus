-- ============================================================================
-- Migration 7: Federation user support
--
-- Adds `server_name` and `is_remote` columns to the `users` table so that
-- shadow accounts for remote-server users can be stored locally.
--
-- A remote user has:
--   is_remote  = TRUE
--   server_name = 'nexus.other.tld'
--
-- Local users have both columns NULL/FALSE (default).
-- ============================================================================

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS server_name TEXT,
    ADD COLUMN IF NOT EXISTS is_remote   BOOLEAN NOT NULL DEFAULT FALSE;

-- Index for efficient lookup of "(username, server)" pairs when receiving
-- an inbound federated friend request.
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_federated
    ON users (LOWER(username), server_name)
    WHERE server_name IS NOT NULL;
