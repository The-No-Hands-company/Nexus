//! Experience profile routes.
//!
//! Enables the "one app" vision where users can choose between:
//! - full Nexus platform experience
//! - messaging-first experience

use axum::{
    Json, Router,
    extract::{Extension, State},
    middleware,
    routing::get,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::module_gating;
use nexus_db::repository::user_experience;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

use crate::{AppState, middleware::AuthContext};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/users/@me/experience-profile",
            get(get_profile).put(update_profile),
        )
        .route_layer(middleware::from_fn(
            crate::middleware::combined_auth_middleware,
        ))
}

#[derive(Debug, Deserialize)]
struct UpdateExperienceRequest {
    experience_mode: Option<String>,
    default_surface: Option<String>,
    enabled_modules: Option<serde_json::Value>,
    compact_navigation: Option<bool>,
    minimal_notifications: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ExperienceProfileResponse {
    user_id: uuid::Uuid,
    experience_mode: String,
    default_surface: String,
    enabled_modules: serde_json::Value,
    effective_enabled_modules: serde_json::Value,
    compact_navigation: bool,
    minimal_notifications: bool,
    updated_at: chrono::DateTime<chrono::Utc>,
    messaging_mode: bool,
}

async fn get_profile(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<ExperienceProfileResponse>> {
    let profile = user_experience::get_or_create(&state.db.pool, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(to_response(profile)))
}

async fn update_profile(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<UpdateExperienceRequest>,
) -> NexusResult<Json<ExperienceProfileResponse>> {
    if let Some(ref mode) = body.experience_mode
        && !matches!(mode.as_str(), "full" | "messaging") {
            return Err(NexusError::Validation {
                message: "experience_mode must be full or messaging".into(),
            });
        }

    if let Some(ref surface) = body.default_surface
        && (surface.trim().is_empty() || surface.len() > 64) {
            return Err(NexusError::Validation {
                message: "default_surface must be 1-64 characters".into(),
            });
        }

    if let Some(ref modules) = body.enabled_modules {
        let Some(arr) = modules.as_array() else {
            return Err(NexusError::Validation {
                message: "enabled_modules must be an array of module IDs".into(),
            });
        };
        if arr.len() > 64 {
            return Err(NexusError::Validation {
                message: "enabled_modules supports at most 64 entries".into(),
            });
        }
        if arr
            .iter()
            .any(|v| v.as_str().map(|s| s.len() > 64).unwrap_or(true))
        {
            return Err(NexusError::Validation {
                message: "enabled_modules entries must be strings up to 64 chars".into(),
            });
        }
    }

    let mode = body.experience_mode.as_deref();
    let default_surface = if body.default_surface.is_some() {
        body.default_surface.as_deref()
    } else if mode == Some("messaging") {
        Some("messages")
    } else {
        None
    };

    let messaging_defaults;
    let enabled_modules = if body.enabled_modules.is_some() {
        body.enabled_modules.as_ref()
    } else if mode == Some("messaging") {
        messaging_defaults = serde_json::to_value(module_gating::default_modules_messaging())
            .unwrap_or_else(|_| serde_json::json!([]));
        Some(&messaging_defaults)
    } else {
        None
    };

    let profile = user_experience::upsert(
        &state.db.pool,
        ctx.user_id,
        mode,
        default_surface,
        enabled_modules,
        body.compact_navigation,
        body.minimal_notifications,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(to_response(profile)))
}

fn to_response(profile: user_experience::UserExperienceProfile) -> ExperienceProfileResponse {
    let messaging_mode = profile.experience_mode == "messaging";

    let effective_enabled_modules = if profile
        .enabled_modules
        .as_array()
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        profile.enabled_modules.clone()
    } else if messaging_mode {
        let modules = module_gating::default_modules_messaging();
        serde_json::to_value(&modules).unwrap_or_else(|_| serde_json::json!([]))
    } else {
        let modules = module_gating::default_modules_full();
        serde_json::to_value(&modules).unwrap_or_else(|_| serde_json::json!([]))
    };

    ExperienceProfileResponse {
        user_id: profile.user_id,
        experience_mode: profile.experience_mode,
        default_surface: profile.default_surface,
        enabled_modules: profile.enabled_modules,
        effective_enabled_modules,
        compact_navigation: profile.compact_navigation,
        minimal_notifications: profile.minimal_notifications,
        updated_at: profile.updated_at,
        messaging_mode,
    }
}
