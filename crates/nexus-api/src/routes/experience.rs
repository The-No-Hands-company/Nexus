//! Experience profile routes.
//!
//! Enables the "one app" vision where users can choose between:
//! - full Nexus platform experience
//! - messaging-first experience

use axum::{
    extract::{Extension, State},
    middleware,
    routing::{get, put},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_db::repository::user_experience;
use serde::Deserialize;
use std::sync::Arc;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/users/@me/experience-profile",
            get(get_profile).put(update_profile),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

#[derive(Debug, Deserialize)]
struct UpdateExperienceRequest {
    experience_mode: Option<String>,
    default_surface: Option<String>,
    enabled_modules: Option<serde_json::Value>,
    compact_navigation: Option<bool>,
    minimal_notifications: Option<bool>,
}

async fn get_profile(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<user_experience::UserExperienceProfile>> {
    let profile = user_experience::get_or_create(&state.db.pool, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(profile))
}

async fn update_profile(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<UpdateExperienceRequest>,
) -> NexusResult<Json<user_experience::UserExperienceProfile>> {
    if let Some(ref mode) = body.experience_mode {
        if !matches!(mode.as_str(), "full" | "messaging") {
            return Err(NexusError::Validation {
                message: "experience_mode must be full or messaging".into(),
            });
        }
    }

    if let Some(ref surface) = body.default_surface {
        if surface.trim().is_empty() || surface.len() > 64 {
            return Err(NexusError::Validation {
                message: "default_surface must be 1-64 characters".into(),
            });
        }
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
        if arr.iter().any(|v| v.as_str().map(|s| s.len() > 64).unwrap_or(true)) {
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
        messaging_defaults = serde_json::json!([
            "messages",
            "dms",
            "calls",
            "contacts",
            "notifications"
        ]);
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

    Ok(Json(profile))
}
