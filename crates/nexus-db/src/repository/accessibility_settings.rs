//! User accessibility settings repository queries.

use nexus_common::models::accessibility::UserAccessibilitySettings;
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::USER_ACCESSIBILITY_COLS;

pub async fn get_settings(
    pool: &AnyPool,
    user_id: Uuid,
) -> Result<Option<UserAccessibilitySettings>, sqlx::Error> {
    let q = format!(
        "SELECT {USER_ACCESSIBILITY_COLS} FROM user_accessibility_settings WHERE user_id = $1"
    );
    sqlx::query_as::<_, UserAccessibilitySettings>(&q)
        .bind(user_id.to_string())
        .fetch_optional(pool)
        .await
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_settings(
    pool: &AnyPool,
    user_id: Uuid,
    screen_reader_mode: Option<bool>,
    announce_messages: Option<bool>,
    announce_reactions: Option<bool>,
    announce_typing: Option<bool>,
    keyboard_shortcuts: Option<bool>,
    high_contrast_mode: Option<bool>,
    reduced_motion: Option<bool>,
    font_family: Option<&str>,
    custom_font_name: Option<&str>,
    color_blind_mode: Option<&str>,
    preferred_language: Option<&str>,
    auto_translate: Option<bool>,
    rtl_override: Option<bool>,
    captions_enabled: Option<bool>,
    caption_font_size: Option<&str>,
    caption_position: Option<&str>,
    tts_enabled: Option<bool>,
    tts_rate: Option<f32>,
    tts_voice: Option<&str>,
) -> Result<UserAccessibilitySettings, sqlx::Error> {
    let q = format!(
        "INSERT INTO user_accessibility_settings (
            user_id, screen_reader_mode, announce_messages, announce_reactions,
            announce_typing, keyboard_shortcuts, high_contrast_mode, reduced_motion,
            font_family, custom_font_name, color_blind_mode,
            preferred_language, auto_translate, rtl_override,
            captions_enabled, caption_font_size, caption_position,
            tts_enabled, tts_rate, tts_voice
        ) VALUES (
            $1,
            COALESCE($2, false), COALESCE($3, true), COALESCE($4, false),
            COALESCE($5, false), COALESCE($6, true), COALESCE($7, false), COALESCE($8, false),
            COALESCE($9, 'system'), $10, COALESCE($11, 'none'),
            COALESCE($12, 'en'), COALESCE($13, false), COALESCE($14, false),
            COALESCE($15, false), COALESCE($16, 'md'), COALESCE($17, 'bottom'),
            COALESCE($18, false), COALESCE($19, 1.0), COALESCE($20, 'default')
        )
        ON CONFLICT (user_id) DO UPDATE SET
            screen_reader_mode  = COALESCE($2, user_accessibility_settings.screen_reader_mode),
            announce_messages   = COALESCE($3, user_accessibility_settings.announce_messages),
            announce_reactions  = COALESCE($4, user_accessibility_settings.announce_reactions),
            announce_typing     = COALESCE($5, user_accessibility_settings.announce_typing),
            keyboard_shortcuts  = COALESCE($6, user_accessibility_settings.keyboard_shortcuts),
            high_contrast_mode  = COALESCE($7, user_accessibility_settings.high_contrast_mode),
            reduced_motion      = COALESCE($8, user_accessibility_settings.reduced_motion),
            font_family         = COALESCE($9, user_accessibility_settings.font_family),
            custom_font_name    = COALESCE($10, user_accessibility_settings.custom_font_name),
            color_blind_mode    = COALESCE($11, user_accessibility_settings.color_blind_mode),
            preferred_language  = COALESCE($12, user_accessibility_settings.preferred_language),
            auto_translate      = COALESCE($13, user_accessibility_settings.auto_translate),
            rtl_override        = COALESCE($14, user_accessibility_settings.rtl_override),
            captions_enabled    = COALESCE($15, user_accessibility_settings.captions_enabled),
            caption_font_size   = COALESCE($16, user_accessibility_settings.caption_font_size),
            caption_position    = COALESCE($17, user_accessibility_settings.caption_position),
            tts_enabled         = COALESCE($18, user_accessibility_settings.tts_enabled),
            tts_rate            = COALESCE($19, user_accessibility_settings.tts_rate),
            tts_voice           = COALESCE($20, user_accessibility_settings.tts_voice),
            updated_at          = NOW()
        RETURNING {USER_ACCESSIBILITY_COLS}"
    );
    let rate_f64 = tts_rate.map(|v| v as f64);
    sqlx::query_as::<_, UserAccessibilitySettings>(&q)
        .bind(user_id.to_string())
        .bind(screen_reader_mode)
        .bind(announce_messages)
        .bind(announce_reactions)
        .bind(announce_typing)
        .bind(keyboard_shortcuts)
        .bind(high_contrast_mode)
        .bind(reduced_motion)
        .bind(font_family)
        .bind(custom_font_name)
        .bind(color_blind_mode)
        .bind(preferred_language)
        .bind(auto_translate)
        .bind(rtl_override)
        .bind(captions_enabled)
        .bind(caption_font_size)
        .bind(caption_position)
        .bind(tts_enabled)
        .bind(rate_f64)
        .bind(tts_voice)
        .fetch_one(pool)
        .await
}
