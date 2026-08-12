-- Record which ecosystem account a user row was provisioned from.
--
-- Accounts now live in Nexus-Auth, and this server sees them only as the `sub`
-- claim of an identity token — a string like `usr-msosh4ui-2`, which is not a
-- UUID and so cannot be `users.id` directly. The id is derived from it
-- deterministically (UUIDv5 under a fixed namespace), which is what makes
-- provisioning idempotent without a read-then-write race.
--
-- This column is not the lookup key; the derived id is. It exists so the
-- mapping is legible to an operator — "which chat user is Auth account X" is
-- otherwise only answerable by recomputing a hash — and so a future change in
-- Auth's id format shows up as a visible duplicate rather than a silent one.
ALTER TABLE users ADD COLUMN external_id TEXT;

-- Unique rather than a plain index: two rows claiming the same ecosystem
-- account is a corruption we want the database to refuse, not something to
-- discover later. Partial, because local rows legitimately have no external id.
CREATE UNIQUE INDEX idx_users_external_id ON users (external_id)
    WHERE external_id IS NOT NULL;
