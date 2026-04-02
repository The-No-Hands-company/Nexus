//! Repository for per-user experience profiles.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{AnyPool, Row};
use uuid::Uuid;

use crate::any_compat::{get_datetime, get_json_value, get_uuid};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserExperienceProfile {
    pub user_id: Uuid,
    pub experience_mode: String,
    pub default_surface: String,
    pub enabled_modules: serde_json::Value,
    pub compact_navigation: bool,
    pub minimal_notifications: bool,
    pub updated_at: DateTime<Utc>,
}

fn row_to_profile(row: &sqlx::any::AnyRow) -> Result<UserExperienceProfile, sqlx::Error> {
    Ok(UserExperienceProfile {
        user_id: get_uuid(row, "user_id")?,
        experience_mode: row.try_get("experience_mode")?,
        default_surface: row.try_get("default_surface")?,
        enabled_modules: get_json_value(row, "enabled_modules")?,
        compact_navigation: row.try_get("compact_navigation")?,
        minimal_notifications: row.try_get("minimal_notifications")?,
        updated_at: get_datetime(row, "updated_at")?,
    })
}

pub async fn get_or_create(
    pool: &AnyPool,
    user_id: Uuid,
) -> Result<UserExperienceProfile, sqlx::Error> {
    sqlx::query(
        "INSERT INTO user_experience_profiles (user_id) VALUES ($1::uuid) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id.to_string())
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT user_id::text AS user_id, experience_mode, default_surface, enabled_modules, \
         compact_navigation, minimal_notifications, updated_at::text AS updated_at \
         FROM user_experience_profiles WHERE user_id = $1::uuid",
    )
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await?;

    row_to_profile(&row)
}

pub async fn upsert(
    pool: &AnyPool,
    user_id: Uuid,
    experience_mode: Option<&str>,
    default_surface: Option<&str>,
    enabled_modules: Option<&serde_json::Value>,
    compact_navigation: Option<bool>,
    minimal_notifications: Option<bool>,
) -> Result<UserExperienceProfile, sqlx::Error> {
    let enabled_modules_json = enabled_modules.map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string()));

    let row = sqlx::query(
        "INSERT INTO user_experience_profiles \
         (user_id, experience_mode, default_surface, enabled_modules, compact_navigation, minimal_notifications) \
         VALUES (\
            $1::uuid, \
            COALESCE($2, 'full'), \
            COALESCE($3, 'home'), \
            COALESCE($4::jsonb, '[]'::jsonb), \
            COALESCE($5, false), \
            COALESCE($6, false)\
         ) \
         ON CONFLICT (user_id) DO UPDATE SET \
            experience_mode = COALESCE($2, user_experience_profiles.experience_mode), \
            default_surface = COALESCE($3, user_experience_profiles.default_surface), \
            enabled_modules = COALESCE($4::jsonb, user_experience_profiles.enabled_modules), \
            compact_navigation = COALESCE($5, user_experience_profiles.compact_navigation), \
            minimal_notifications = COALESCE($6, user_experience_profiles.minimal_notifications), \
            updated_at = NOW() \
         RETURNING user_id::text AS user_id, experience_mode, default_surface, enabled_modules, \
                   compact_navigation, minimal_notifications, updated_at::text AS updated_at",
    )
    .bind(user_id.to_string())
    .bind(experience_mode)
    .bind(default_surface)
    .bind(enabled_modules_json)
    .bind(compact_navigation)
    .bind(minimal_notifications)
    .fetch_one(pool)
    .await?;

    row_to_profile(&row)
}
