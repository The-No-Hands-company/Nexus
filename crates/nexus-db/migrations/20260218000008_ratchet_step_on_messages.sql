-- v0.5.1 — Signal Protocol ratchet step tracking on encrypted messages.
--
-- Adds `sender_ratchet_step` to `encrypted_messages` so that recipients can
-- detect whether any ratchet steps were skipped (out-of-order or dropped
-- messages) without the server needing to understand the ratchet itself.
--
-- The column is nullable so existing rows remain valid.

ALTER TABLE encrypted_messages
    ADD COLUMN IF NOT EXISTS sender_ratchet_step INTEGER;

-- Index to support "give me all messages sent after ratchet step N from
-- device D in channel C" queries used during session resumption / catch-up.
CREATE INDEX IF NOT EXISTS idx_enc_msg_ratchet
    ON encrypted_messages(channel_id, sender_device_id, sender_ratchet_step)
    WHERE sender_ratchet_step IS NOT NULL;
