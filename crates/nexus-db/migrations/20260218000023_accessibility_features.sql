-- Phase 18: Accessibility & Inclusivity (v1.7)
-- 18-01 Screen Reader Enhancements
-- 18-02 Visual Accessibility
-- 18-03 Language & Communication
-- 18-04 Voice Accessibility

-- ─── User Accessibility Preferences ──────────────────────────────────────────
-- Centralised per-user accessibility settings (server-synced, not just localStorage).
CREATE TABLE IF NOT EXISTS user_accessibility_settings (
    user_id              UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,

    -- 18-01 Screen reader
    screen_reader_mode   BOOLEAN NOT NULL DEFAULT false,
    announce_messages     BOOLEAN NOT NULL DEFAULT true,   -- announce new messages
    announce_reactions    BOOLEAN NOT NULL DEFAULT false,  -- announce reaction events
    announce_typing       BOOLEAN NOT NULL DEFAULT false,  -- announce typing indicators
    keyboard_shortcuts    BOOLEAN NOT NULL DEFAULT true,

    -- 18-02 Visual
    high_contrast_mode   BOOLEAN NOT NULL DEFAULT false,
    reduced_motion       BOOLEAN NOT NULL DEFAULT false,
    font_family          TEXT NOT NULL DEFAULT 'system',  -- system | monospace | dyslexic | serif | custom
    custom_font_name     TEXT,                            -- when font_family='custom'
    color_blind_mode     TEXT NOT NULL DEFAULT 'none',    -- none | protanopia | deuteranopia | tritanopia

    -- 18-03 Language
    preferred_language   TEXT NOT NULL DEFAULT 'en',      -- ISO 639-1
    auto_translate       BOOLEAN NOT NULL DEFAULT false,
    rtl_override         BOOLEAN NOT NULL DEFAULT false,  -- force RTL layout

    -- 18-04 Voice accessibility
    captions_enabled     BOOLEAN NOT NULL DEFAULT false,
    caption_font_size    TEXT NOT NULL DEFAULT 'md',      -- sm | md | lg | xl
    caption_position     TEXT NOT NULL DEFAULT 'bottom',  -- top | bottom
    tts_enabled          BOOLEAN NOT NULL DEFAULT false,
    tts_rate             REAL NOT NULL DEFAULT 1.0,       -- speech rate 0.5-2.0
    tts_voice            TEXT NOT NULL DEFAULT 'default', -- browser/OS voice identifier

    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─── Voice Channel Captions ──────────────────────────────────────────────────
-- Stores live caption segments for voice channels (on-device speech recognition pushed to server).
CREATE TABLE IF NOT EXISTS voice_captions (
    id                   UUID PRIMARY KEY,
    channel_id           UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    speaker_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    text                 TEXT NOT NULL,
    language             TEXT NOT NULL DEFAULT 'en',
    is_final             BOOLEAN NOT NULL DEFAULT false,  -- false = interim, true = finalised
    started_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    ended_at             TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_voice_captions_channel ON voice_captions(channel_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_voice_captions_speaker ON voice_captions(speaker_id, created_at DESC);

-- ─── Message TTS Requests ────────────────────────────────────────────────────
-- Track which messages the user wants read aloud (ephemeral, auto-purged).
CREATE TABLE IF NOT EXISTS message_tts_requests (
    id                   UUID PRIMARY KEY,
    user_id              UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message_id           UUID NOT NULL,
    channel_id           UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    status               TEXT NOT NULL DEFAULT 'pending',  -- pending | playing | done
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_tts_requests_user ON message_tts_requests(user_id, created_at DESC);

-- ─── Translation Cache ───────────────────────────────────────────────────────
-- Caches on-device translations for messages to avoid re-translating.
CREATE TABLE IF NOT EXISTS message_translations (
    id                   UUID PRIMARY KEY,
    message_id           UUID NOT NULL,
    source_language      TEXT NOT NULL,
    target_language      TEXT NOT NULL,
    translated_text      TEXT NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(message_id, target_language)
);

CREATE INDEX IF NOT EXISTS idx_translations_message ON message_translations(message_id);
