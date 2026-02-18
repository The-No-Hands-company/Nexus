-- v0.5 End-to-End Encryption — Signal Protocol key infrastructure
--
-- Key hierarchy:
--   IdentityKey       — long-lived Ed25519 keypair per device (public stored here)
--   SignedPreKey      — medium-lived X25519 Diffie-Hellman key, signed by IdentityKey
--   OneTimePreKey     — single-use X25519 keys (key bundle for X3DH)
--   Session           — derived session state stored encrypted, keyed per (sender_device, recipient_device)
--
-- Encrypted messages store only ciphertext + metadata; server never sees plaintext.

-- -----------------------------------------------------------------------
-- Devices — each user can have multiple verified devices
-- -----------------------------------------------------------------------
CREATE TABLE devices (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Human-readable label ("iPhone 15", "Firefox on Linux")
    name            TEXT NOT NULL DEFAULT '',
    -- Ed25519 public identity key, base64-encoded
    identity_key    TEXT NOT NULL,
    -- Signed pre-key (X25519 public), base64-encoded
    signed_pre_key          TEXT NOT NULL,
    -- Signature over signed_pre_key by identity_key, base64-encoded
    signed_pre_key_sig      TEXT NOT NULL,
    -- Signed pre-key ID (client-assigned monotonic integer)
    signed_pre_key_id       INTEGER NOT NULL DEFAULT 0,
    -- Device type for UX hints
    device_type     TEXT NOT NULL DEFAULT 'unknown'
                        CHECK (device_type IN ('desktop', 'mobile', 'browser', 'unknown')),
    -- Last time this device was seen making an API call
    last_seen_at    TIMESTAMPTZ,
    -- Whether another device has verified this one
    verified        BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, identity_key)
);

CREATE INDEX idx_devices_user_id ON devices(user_id);

-- -----------------------------------------------------------------------
-- One-Time Pre-Keys — consumed during X3DH key agreement
-- -----------------------------------------------------------------------
CREATE TABLE one_time_pre_keys (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id   UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    -- Client-assigned key ID
    key_id      INTEGER NOT NULL,
    -- X25519 public key, base64-encoded
    public_key  TEXT NOT NULL,
    -- True once consumed by a key exchange
    consumed    BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (device_id, key_id)
);

CREATE INDEX idx_otpk_device_unconsumed ON one_time_pre_keys(device_id) WHERE NOT consumed;

-- -----------------------------------------------------------------------
-- E2EE Sessions — per (sender_device, recipient_device) double ratchet state
-- -----------------------------------------------------------------------
CREATE TABLE e2ee_sessions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The device that initiated / owns this session record
    owner_device_id     UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    -- The remote device this session is with
    remote_device_id    UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    -- Opaque session state blob (AES-256-GCM encrypted by device, base64)
    session_state       TEXT NOT NULL,
    -- Our local ratchet step counter (for ordering / debugging)
    ratchet_step        INTEGER NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_device_id, remote_device_id)
);

CREATE INDEX idx_e2ee_sessions_owner ON e2ee_sessions(owner_device_id);

-- -----------------------------------------------------------------------
-- Encrypted Messages — ciphertext-only storage
-- -----------------------------------------------------------------------
CREATE TABLE encrypted_messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Which DM channel or E2EE channel this belongs to
    channel_id      UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    sender_id       UUID NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    sender_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE SET NULL,
    -- Per-recipient ciphertext envelope, keyed by recipient_device_id UUID
    -- { "device_uuid": { "type": 1|2, "body": "<base64 ciphertext>" } }
    ciphertext_map  JSONB NOT NULL DEFAULT '{}',
    -- Optional: encrypted filename + size hint for attachments
    attachment_meta JSONB,
    -- Server-side sort key (monotonic)
    sequence        BIGINT NOT NULL DEFAULT 0,
    -- Client-set timestamp (not trusted for ordering)
    client_ts       TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_enc_msg_channel ON encrypted_messages(channel_id, created_at DESC);
CREATE INDEX idx_enc_msg_sender  ON encrypted_messages(sender_id);

-- -----------------------------------------------------------------------
-- E2EE Channel Config — marks a channel as end-to-end encrypted
-- -----------------------------------------------------------------------
CREATE TABLE e2ee_channels (
    channel_id      UUID PRIMARY KEY REFERENCES channels(id) ON DELETE CASCADE,
    -- Who enabled E2EE and when
    enabled_by      UUID NOT NULL REFERENCES users(id),
    enabled_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Rotation policy: how often the shared key should be rotated (seconds)
    rotation_interval_secs  INTEGER NOT NULL DEFAULT 604800, -- 7 days
    last_rotated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- -----------------------------------------------------------------------
-- Device Verification Records — safety numbers / QR verification
-- -----------------------------------------------------------------------
CREATE TABLE device_verifications (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The user doing the verifying
    verifier_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- The device being verified
    target_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    -- Method used: 'safety_number', 'qr_scan', 'emoji'
    method          TEXT NOT NULL DEFAULT 'safety_number'
                        CHECK (method IN ('safety_number', 'qr_scan', 'emoji')),
    verified_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (verifier_id, target_device_id)
);

CREATE INDEX idx_device_verif_verifier ON device_verifications(verifier_id);
CREATE INDEX idx_device_verif_target   ON device_verifications(target_device_id);

-- -----------------------------------------------------------------------
-- Triggers
-- -----------------------------------------------------------------------
CREATE TRIGGER set_updated_at_devices
    BEFORE UPDATE ON devices
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER set_updated_at_e2ee_sessions
    BEFORE UPDATE ON e2ee_sessions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
