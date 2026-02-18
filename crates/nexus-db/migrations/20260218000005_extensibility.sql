-- Nexus Database Schema
-- Migration: v0.7 Extensibility — bots, webhooks, slash commands, themes, plugins
-- Generated for PostgreSQL 15+

-- ============================================================================
-- Bot Applications
-- ============================================================================

CREATE TABLE bot_applications (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            VARCHAR(100) NOT NULL,
    description     VARCHAR(1000),
    avatar          TEXT,
    -- The hashed bot token (prefixed "Bot " in HTTP headers)
    token_hash      TEXT NOT NULL,
    -- Public key for webhook signature verification (Ed25519 hex)
    public_key      VARCHAR(128) NOT NULL,
    -- OAuth2 redirect URIs for the bot
    redirect_uris   JSONB NOT NULL DEFAULT '[]',
    -- Permissions bitfield the bot requests
    permissions     BIGINT NOT NULL DEFAULT 0,
    -- Whether the bot is verified (higher rate limits, public)
    verified        BOOLEAN NOT NULL DEFAULT false,
    -- Whether the bot is public (anyone can add it) or private (owner only)
    is_public       BOOLEAN NOT NULL DEFAULT false,
    -- Whether the bot participates in interactions (slash commands etc.)
    interactions_endpoint_url TEXT,
    flags           BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bot_applications_owner ON bot_applications (owner_id);
CREATE INDEX idx_bot_applications_name ON bot_applications (LOWER(name));

-- ============================================================================
-- Bot Members (bots installed into servers)
-- ============================================================================

CREATE TABLE bot_server_installs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bot_id          UUID NOT NULL REFERENCES bot_applications(id) ON DELETE CASCADE,
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    installed_by    UUID NOT NULL REFERENCES users(id),
    -- Scopes granted to this install (e.g., ["bot", "applications.commands"])
    scopes          JSONB NOT NULL DEFAULT '["bot"]',
    -- Permissions bitfield for this server
    permissions     BIGINT NOT NULL DEFAULT 0,
    installed_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (bot_id, server_id)
);

CREATE INDEX idx_bot_installs_server ON bot_server_installs (server_id);
CREATE INDEX idx_bot_installs_bot ON bot_server_installs (bot_id);

-- ============================================================================
-- Webhooks
-- ============================================================================

CREATE TABLE webhooks (
    id              UUID PRIMARY KEY,
    -- 'incoming' or 'outgoing'
    webhook_type    VARCHAR(20) NOT NULL DEFAULT 'incoming',
    server_id       UUID REFERENCES servers(id) ON DELETE CASCADE,
    channel_id      UUID REFERENCES channels(id) ON DELETE CASCADE,
    creator_id      UUID REFERENCES users(id) ON DELETE SET NULL,
    name            VARCHAR(100) NOT NULL,
    avatar          TEXT,
    -- Secret token for incoming webhook URLs / HMAC-signing outgoing
    token           TEXT NOT NULL,
    -- For outgoing webhooks: the URL to POST events to
    url             TEXT,
    -- Which events to fire for outgoing webhooks (JSON array of event names)
    events          JSONB NOT NULL DEFAULT '[]',
    -- Whether the webhook is active
    active          BOOLEAN NOT NULL DEFAULT true,
    -- Number of times this webhook has fired (informational)
    delivery_count  BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhooks_channel ON webhooks (channel_id) WHERE channel_id IS NOT NULL;
CREATE INDEX idx_webhooks_server ON webhooks (server_id) WHERE server_id IS NOT NULL;
CREATE INDEX idx_webhooks_type ON webhooks (webhook_type, active);

-- ============================================================================
-- Slash Commands
-- ============================================================================

CREATE TABLE slash_commands (
    id              UUID PRIMARY KEY,
    application_id  UUID NOT NULL REFERENCES bot_applications(id) ON DELETE CASCADE,
    -- NULL = global command, non-null = server-scoped command
    server_id       UUID REFERENCES servers(id) ON DELETE CASCADE,
    name            VARCHAR(32) NOT NULL,
    -- Localized name map {locale: name}
    name_localizations JSONB,
    description     VARCHAR(100) NOT NULL,
    description_localizations JSONB,
    -- JSON definition of options (subcommands, params, types, choices)
    options         JSONB NOT NULL DEFAULT '[]',
    -- Default member permissions required to use the command (bitfield as text)
    default_member_permissions TEXT,
    -- Whether the command is enabled in DMs
    dm_permission   BOOLEAN NOT NULL DEFAULT true,
    -- 1 = CHAT_INPUT, 2 = USER, 3 = MESSAGE
    command_type    INTEGER NOT NULL DEFAULT 1,
    -- Version hash for cache invalidation
    version         UUID NOT NULL DEFAULT gen_random_uuid(),
    -- Soft-delete / disable without removing
    enabled         BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (application_id, server_id, name, command_type)
);

CREATE INDEX idx_slash_commands_app ON slash_commands (application_id);
CREATE INDEX idx_slash_commands_server ON slash_commands (server_id) WHERE server_id IS NOT NULL;
CREATE INDEX idx_slash_commands_global ON slash_commands (application_id) WHERE server_id IS NULL;
CREATE INDEX idx_slash_commands_name ON slash_commands (LOWER(name));

-- ============================================================================
-- Interaction Log (audit trail + deduplication)
-- ============================================================================

CREATE TABLE interactions (
    id              UUID PRIMARY KEY,
    -- 'APPLICATION_COMMAND', 'MESSAGE_COMPONENT', 'AUTOCOMPLETE', 'MODAL_SUBMIT'
    interaction_type VARCHAR(40) NOT NULL,
    application_id  UUID NOT NULL REFERENCES bot_applications(id) ON DELETE CASCADE,
    command_id      UUID REFERENCES slash_commands(id) ON DELETE SET NULL,
    channel_id      UUID REFERENCES channels(id) ON DELETE SET NULL,
    server_id       UUID REFERENCES servers(id) ON DELETE SET NULL,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Raw interaction data snapshot
    data            JSONB NOT NULL DEFAULT '{}',
    -- Acknowledgement token (used to respond within 3s window)
    token           TEXT NOT NULL,
    -- 'pending', 'acknowledged', 'responded', 'expired'
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Interactions expire after 15 minutes
    expires_at      TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '15 minutes'
);

CREATE INDEX idx_interactions_app ON interactions (application_id);
CREATE INDEX idx_interactions_user ON interactions (user_id);
CREATE INDEX idx_interactions_expires ON interactions (expires_at) WHERE status = 'pending';

-- ============================================================================
-- Client Plugins
-- ============================================================================

CREATE TABLE client_plugins (
    id              UUID PRIMARY KEY,
    author_id       UUID REFERENCES users(id) ON DELETE SET NULL,
    name            VARCHAR(100) NOT NULL,
    slug            VARCHAR(60) NOT NULL UNIQUE,
    version         VARCHAR(20) NOT NULL,
    description     VARCHAR(500),
    homepage        TEXT,
    repository      TEXT,
    -- Semver range of Nexus client versions this plugin supports
    engine_range    VARCHAR(30) NOT NULL DEFAULT '>=0.7.0',
    -- Permissions the plugin requests (JSON array)
    permissions     JSONB NOT NULL DEFAULT '[]',
    -- Bundled plugin JS (stored as text — kept small; large plugins use CDN URL)
    bundle_url      TEXT,
    bundle_hash     VARCHAR(64),
    -- Whether the plugin has been reviewed and is safe to show in the marketplace
    verified        BOOLEAN NOT NULL DEFAULT false,
    -- Whether it's currently available for install
    active          BOOLEAN NOT NULL DEFAULT true,
    install_count   INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_plugins_slug ON client_plugins (slug);
CREATE INDEX idx_plugins_verified ON client_plugins (verified, active, install_count DESC);

-- ============================================================================
-- User Plugin Installs
-- ============================================================================

CREATE TABLE user_plugin_installs (
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    plugin_id   UUID NOT NULL REFERENCES client_plugins(id) ON DELETE CASCADE,
    enabled     BOOLEAN NOT NULL DEFAULT true,
    settings    JSONB NOT NULL DEFAULT '{}',
    installed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, plugin_id)
);

CREATE INDEX idx_user_plugins_user ON user_plugin_installs (user_id, enabled);

-- ============================================================================
-- Custom Themes
-- ============================================================================

CREATE TABLE themes (
    id              UUID PRIMARY KEY,
    author_id       UUID REFERENCES users(id) ON DELETE SET NULL,
    name            VARCHAR(100) NOT NULL,
    slug            VARCHAR(60) NOT NULL UNIQUE,
    version         VARCHAR(20) NOT NULL,
    description     VARCHAR(500),
    -- CSS variables map stored as JSON {--var-name: value}
    variables       JSONB NOT NULL DEFAULT '{}',
    -- Raw CSS overrides blob
    css             TEXT NOT NULL DEFAULT '',
    -- Preview screenshot URL
    preview_url     TEXT,
    verified        BOOLEAN NOT NULL DEFAULT false,
    active          BOOLEAN NOT NULL DEFAULT true,
    install_count   INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_themes_slug ON themes (slug);
CREATE INDEX idx_themes_verified ON themes (verified, active, install_count DESC);

-- ============================================================================
-- User Theme Installs
-- ============================================================================

CREATE TABLE user_theme_installs (
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    theme_id    UUID NOT NULL REFERENCES themes(id) ON DELETE CASCADE,
    active      BOOLEAN NOT NULL DEFAULT false,  -- only one theme active at a time
    installed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, theme_id)
);

CREATE INDEX idx_user_themes ON user_theme_installs (user_id, active);
