//! Accessibility settings routes — 18-01/02/03/04.
//!
//! GET   /users/@me/accessibility  — Get user accessibility settings
//! PATCH /users/@me/accessibility  — Update user accessibility settings

use axum::{
    Json, Router,
    extract::{Extension, State},
    middleware,
    routing::get,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_db::repository::accessibility_settings;
use serde::Deserialize;
use std::sync::Arc;

use crate::{AppState, middleware::AuthContext};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/users/@me/accessibility",
            get(get_settings).patch(update_settings),
        )
        .route_layer(middleware::from_fn(
            crate::middleware::combined_auth_middleware,
        ))
}

// ── Request bodies ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct UpdateAccessibilityRequest {
    // Screen reader
    screen_reader_mode: Option<bool>,
    announce_messages: Option<bool>,
    announce_reactions: Option<bool>,
    announce_typing: Option<bool>,
    keyboard_shortcuts: Option<bool>,

    // Visual
    high_contrast_mode: Option<bool>,
    reduced_motion: Option<bool>,
    font_family: Option<String>,
    custom_font_name: Option<String>,
    color_blind_mode: Option<String>,

    // Language
    preferred_language: Option<String>,
    auto_translate: Option<bool>,
    rtl_override: Option<bool>,

    // Voice
    captions_enabled: Option<bool>,
    caption_font_size: Option<String>,
    caption_position: Option<String>,
    tts_enabled: Option<bool>,
    tts_rate: Option<f32>,
    tts_voice: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn get_settings(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<nexus_common::models::accessibility::UserAccessibilitySettings>> {
    let settings = accessibility_settings::get_settings(&state.db.pool, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    match settings {
        Some(s) => Ok(Json(s)),
        None => {
            // Return defaults by upserting with all None values
            let s = accessibility_settings::upsert_settings(
                &state.db.pool,
                ctx.user_id,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| NexusError::Internal(e.into()))?;
            Ok(Json(s))
        }
    }
}

async fn update_settings(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<UpdateAccessibilityRequest>,
) -> NexusResult<Json<nexus_common::models::accessibility::UserAccessibilitySettings>> {
    // Validate font_family if provided
    if let Some(ref ff) = body.font_family {
        let valid = ["system", "monospace", "dyslexic", "serif", "custom"];
        if !valid.contains(&ff.as_str()) {
            return Err(NexusError::Validation {
                message: format!("font_family must be one of: {}", valid.join(", ")),
            });
        }
    }

    // Validate color_blind_mode if provided
    if let Some(ref cb) = body.color_blind_mode {
        let valid = ["none", "protanopia", "deuteranopia", "tritanopia"];
        if !valid.contains(&cb.as_str()) {
            return Err(NexusError::Validation {
                message: format!("color_blind_mode must be one of: {}", valid.join(", ")),
            });
        }
    }

    // Validate caption_font_size
    if let Some(ref cfs) = body.caption_font_size {
        let valid = ["sm", "md", "lg", "xl"];
        if !valid.contains(&cfs.as_str()) {
            return Err(NexusError::Validation {
                message: "caption_font_size must be sm, md, lg, or xl".into(),
            });
        }
    }

    // Validate caption_position
    if let Some(ref cp) = body.caption_position
        && cp != "top" && cp != "bottom" {
            return Err(NexusError::Validation {
                message: "caption_position must be top or bottom".into(),
            });
        }

    // Validate tts_rate
    if let Some(rate) = body.tts_rate
        && !(0.5..=2.0).contains(&rate) {
            return Err(NexusError::Validation {
                message: "tts_rate must be between 0.5 and 2.0".into(),
            });
        }

    // Validate preferred_language is 2–5 chars
    if let Some(ref lang) = body.preferred_language
        && (lang.len() < 2 || lang.len() > 5) {
            return Err(NexusError::Validation {
                message: "preferred_language must be a valid ISO 639-1 code (2-5 chars)".into(),
            });
        }

    let s = accessibility_settings::upsert_settings(
        &state.db.pool,
        ctx.user_id,
        body.screen_reader_mode,
        body.announce_messages,
        body.announce_reactions,
        body.announce_typing,
        body.keyboard_shortcuts,
        body.high_contrast_mode,
        body.reduced_motion,
        body.font_family.as_deref(),
        body.custom_font_name.as_deref(),
        body.color_blind_mode.as_deref(),
        body.preferred_language.as_deref(),
        body.auto_translate,
        body.rtl_override,
        body.captions_enabled,
        body.caption_font_size.as_deref(),
        body.caption_position.as_deref(),
        body.tts_enabled,
        body.tts_rate,
        body.tts_voice.as_deref(),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(s))
}
