-- Lite-mode twin of the Postgres migration of the same name; see that file for
-- why rotation needs a clock of its own rather than reusing `updated_at`.
--
-- TEXT, matching every other timestamp in the lite schema (SQLite has no
-- native timestamp type and this table already stores created_at/updated_at
-- that way).
ALTER TABLE devices ADD COLUMN signed_pre_key_rotated_at TEXT;

UPDATE devices SET signed_pre_key_rotated_at = created_at
    WHERE signed_pre_key_rotated_at IS NULL;
