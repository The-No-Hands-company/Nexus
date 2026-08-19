-- When this device's signed pre-key was last replaced.
--
-- `updated_at` cannot answer this. It moves whenever anything about the row
-- changes — a rename, a verification, a last-seen touch — so a device that has
-- not rotated its pre-key in a year can still look freshly updated. Rotation
-- needs its own clock or it cannot be reasoned about at all.
--
-- Why it matters: a signed pre-key is long-lived by design, and the longer one
-- stays in place the more traffic a single compromised key can retroactively
-- expose. Signal's answer is periodic rotation by the client. The server's job
-- is not to force it — refusing to serve a stale bundle would break messaging
-- entirely, and a stale key is worth more than no key — but to make the age
-- visible so a peer can judge it.
ALTER TABLE devices ADD COLUMN signed_pre_key_rotated_at TIMESTAMPTZ;

-- Backfill from creation: a device that has never rotated has held its
-- original key since it registered, which is exactly the age we want reported.
-- Leaving these NULL would make "never rotated" indistinguishable from "column
-- added after this row existed".
UPDATE devices SET signed_pre_key_rotated_at = created_at
    WHERE signed_pre_key_rotated_at IS NULL;
