-- Record which ecosystem account a user row was provisioned from.
-- See the Postgres copy of this migration for the reasoning; SQLite differs
-- only in that ALTER TABLE cannot add a UNIQUE column, so the constraint is a
-- separate partial unique index (which SQLite does support).
ALTER TABLE users ADD COLUMN external_id TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_external_id ON users (external_id)
    WHERE external_id IS NOT NULL;
