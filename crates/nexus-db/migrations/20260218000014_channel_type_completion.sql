-- Migration 014: Channel Type Completion (Phase 12)
-- Adds tables and columns needed to expose forum, announcement, stage channel
-- APIs and group DM management as first-class API surfaces.
-- All `channel_type` ENUM values (forum, announcement, stage, group_dm) already
-- exist from migration 001 — this migration adds the supporting structures.

-- ============================================================
-- Group DM icon support
-- icon column for group_dm channels (name already exists on channels)
-- ============================================================
ALTER TABLE channels ADD COLUMN IF NOT EXISTS icon TEXT;

-- owner_id on channels — identifies the group DM owner / stage host
ALTER TABLE channels ADD COLUMN IF NOT EXISTS owner_id UUID REFERENCES users(id) ON DELETE SET NULL;

-- ============================================================
-- Forum tags — named tag definitions scoped to a forum channel
-- Each forum channel maintains its own tag list.
-- The `tags` column on `threads` stores the *names* of applied tags.
-- ============================================================
CREATE TABLE IF NOT EXISTS forum_tags (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id  UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    emoji       TEXT,           -- optional emoji prefix (unicode or :custom_emoji_name:)
    moderated   BOOLEAN NOT NULL DEFAULT false,  -- only moderators can apply
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (channel_id, name)
);

CREATE INDEX IF NOT EXISTS idx_forum_tags_channel ON forum_tags (channel_id);

-- ============================================================
-- Stage instances — live stage metadata for stage channels
-- ============================================================
CREATE TABLE IF NOT EXISTS stage_instances (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id      UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE UNIQUE,
    guild_id        UUID REFERENCES servers(id) ON DELETE CASCADE,
    topic           TEXT NOT NULL DEFAULT '',
    -- privacy_level: 'guild_only' | 'public'
    privacy_level   TEXT NOT NULL DEFAULT 'guild_only' CHECK (privacy_level IN ('guild_only', 'public')),
    -- Arrays of user IDs stored as jsonb for portability across drivers
    speaker_ids     JSONB NOT NULL DEFAULT '[]',
    hand_raised_ids JSONB NOT NULL DEFAULT '[]',
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at        TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_stage_instances_channel ON stage_instances (channel_id);
CREATE INDEX IF NOT EXISTS idx_stage_instances_guild   ON stage_instances (guild_id) WHERE guild_id IS NOT NULL;

-- ============================================================
-- Channel followers — announcement channel subscriptions
-- A server channel can "follow" an announcement channel so that
-- published (crossposted) messages are automatically relayed there.
-- ============================================================
CREATE TABLE IF NOT EXISTS channel_followers (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The announcement channel being followed
    source_channel_id   UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    -- The target channel that receives crossposted messages
    target_channel_id   UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    -- Server that owns the target channel
    target_guild_id     UUID REFERENCES servers(id) ON DELETE CASCADE,
    -- Webhook used to deliver crosspost messages (nullable — direct insert for local)
    webhook_id          UUID,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (source_channel_id, target_channel_id)
);

CREATE INDEX IF NOT EXISTS idx_channel_followers_source ON channel_followers (source_channel_id);
CREATE INDEX IF NOT EXISTS idx_channel_followers_target ON channel_followers (target_channel_id);

-- ============================================================
-- Crossposted message tracking
-- Records messages that have been published via crosspost so we
-- can enforce the "can only publish once" rule and surface the
-- published flag in message responses.
-- ============================================================
ALTER TABLE messages ADD COLUMN IF NOT EXISTS flags INT NOT NULL DEFAULT 0;
-- Flag bit 1 = crossposted (published from announcement channel)
-- Flag bit 2 = is_crosspost (delivered via crosspost, not original)
-- Flag bit 4 = suppress_embeds
