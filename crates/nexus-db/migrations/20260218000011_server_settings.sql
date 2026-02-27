-- Migration: per-server settings columns
--
-- Adds three configurable columns to `servers`:
--   require_2fa        — members must have TOTP enabled to join
--   spam_window_secs   — rolling window for spam detection (default 30 s)
--   spam_max_messages  — max identical messages in window (default 3)

ALTER TABLE servers
    ADD COLUMN IF NOT EXISTS require_2fa       BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS spam_window_secs  INTEGER NOT NULL DEFAULT 30,
    ADD COLUMN IF NOT EXISTS spam_max_messages INTEGER NOT NULL DEFAULT 3;
