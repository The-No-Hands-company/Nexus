-- Per-user experience mode profile.
-- Supports "one app" with selectable UX surfaces (full platform vs messaging-first).

CREATE TABLE IF NOT EXISTS user_experience_profiles (
    user_id uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    experience_mode text NOT NULL DEFAULT 'full' CHECK (experience_mode IN ('full', 'messaging')),
    default_surface text NOT NULL DEFAULT 'home',
    enabled_modules jsonb NOT NULL DEFAULT '[]'::jsonb,
    compact_navigation boolean NOT NULL DEFAULT false,
    minimal_notifications boolean NOT NULL DEFAULT false,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_user_experience_profiles_mode
    ON user_experience_profiles (experience_mode);
