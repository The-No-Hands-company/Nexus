-- Per-user experience mode profile (SQLite lite mode).

CREATE TABLE IF NOT EXISTS user_experience_profiles (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    experience_mode TEXT NOT NULL DEFAULT 'full' CHECK (experience_mode IN ('full', 'messaging')),
    default_surface TEXT NOT NULL DEFAULT 'home',
    enabled_modules TEXT NOT NULL DEFAULT '[]',
    compact_navigation INTEGER NOT NULL DEFAULT 0,
    minimal_notifications INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_user_experience_profiles_mode
    ON user_experience_profiles (experience_mode);
