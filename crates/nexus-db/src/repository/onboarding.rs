//! Repository for user onboarding progress.

use nexus_common::models::ecosystem::OnboardingProgress;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::ONBOARDING_PROGRESS_COLS;

pub async fn get_progress(
    pool: &AnyPool,
    user_id: Uuid,
) -> Result<Option<OnboardingProgress>, sqlx::Error> {
    let q = format!(
        "SELECT {ONBOARDING_PROGRESS_COLS} FROM onboarding_progress WHERE user_id = $1"
    );
    sqlx::query_as::<_, OnboardingProgress>(&q)
        .bind(user_id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn upsert_progress(
    pool: &AnyPool,
    user_id: Uuid,
    completed_steps: &serde_json::Value,
    dismissed: bool,
) -> Result<OnboardingProgress, sqlx::Error> {
    let q = format!(
        "INSERT INTO onboarding_progress (user_id, completed_steps, dismissed) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (user_id) DO UPDATE SET \
           completed_steps = $2, dismissed = $3, updated_at = now() \
         RETURNING {ONBOARDING_PROGRESS_COLS}"
    );
    sqlx::query_as::<_, OnboardingProgress>(&q)
        .bind(user_id.to_string())
        .bind(serde_json::to_string(completed_steps).unwrap_or_default())
        .bind(dismissed)
        .fetch_one(pool)
        .await
}
