-- v1.9.x Scylla dual-write outbox for messages

CREATE TABLE IF NOT EXISTS scylla_message_outbox (
    id            BIGSERIAL PRIMARY KEY,
    operation     TEXT NOT NULL, -- upsert | delete
    message_id    UUID NOT NULL,
    payload       JSONB,
    status        TEXT NOT NULL DEFAULT 'pending', -- pending | dispatched | processed | failed
    attempt_count INT  NOT NULL DEFAULT 0,
    last_error    TEXT,
    dispatched_at TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_scylla_outbox_status_created
    ON scylla_message_outbox(status, created_at);

CREATE INDEX IF NOT EXISTS idx_scylla_outbox_message
    ON scylla_message_outbox(message_id, created_at DESC);
