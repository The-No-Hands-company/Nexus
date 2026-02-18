//! Plugin & theme routes — marketplace listing, installs, and user settings.

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::plugin::{
        ClientPlugin, SubmitPluginRequest, SubmitThemeRequest, Theme, UpdatePluginSettingsRequest,
        UserPluginInstall, UserThemeInstall,
    },
    snowflake,
};
use nexus_db::repository::plugins;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

/// Plugin and theme routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Plugin marketplace
        .route("/plugins", get(list_plugins).post(submit_plugin))
        .route("/plugins/{slug}", get(get_plugin))
        // User plugin installs
        .route(
            "/users/@me/plugins",
            get(get_my_plugins).post(install_plugin),
        )
        .route(
            "/users/@me/plugins/{plugin_id}",
            delete(uninstall_plugin).patch(update_plugin_settings),
        )
        // Theme marketplace
        .route("/themes", get(list_themes).post(submit_theme))
        .route("/themes/{slug}", get(get_theme))
        // User theme installs
        .route("/users/@me/themes", get(get_my_themes).post(install_theme))
        .route("/users/@me/themes/{theme_id}", delete(uninstall_theme))
        .route("/users/@me/themes/{theme_id}/activate", post(activate_theme))
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

// ============================================================================
// Plugin Marketplace
// ============================================================================

#[derive(Deserialize)]
struct PaginationQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    20
}

/// GET /api/v1/plugins
async fn list_plugins(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(q): Query<PaginationQuery>,
) -> NexusResult<Json<Vec<ClientPlugin>>> {
    let limit = q.limit.min(100);
    let items = plugins::list_plugins(&state.db.pg, limit, q.offset).await?;
    Ok(Json(items))
}

/// GET /api/v1/plugins/{slug}
async fn get_plugin(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> NexusResult<Json<ClientPlugin>> {
    let plugin = plugins::get_plugin_by_slug(&state.db.pg, &slug)
        .await?
        .ok_or(NexusError::NotFound { resource: "plugin".to_string() })?;
    Ok(Json(plugin))
}

/// POST /api/v1/plugins — Submit a new plugin (pending review).
async fn submit_plugin(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<SubmitPluginRequest>,
) -> NexusResult<Json<ClientPlugin>> {
    let id = snowflake::generate_id();
    let plugin = plugins::create_plugin(
        &state.db.pg,
        id,
        Some(auth.user_id),
        &body.name,
        &body.slug,
        body.description.as_deref(),
        &body.version,
        Some(body.bundle_url.as_str()),
        Some(body.bundle_hash.as_str()),
        &body.permissions,
    )
    .await?;
    Ok(Json(plugin))
}

// ============================================================================
// User Plugin Installs
// ============================================================================

/// GET /api/v1/users/@me/plugins
async fn get_my_plugins(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<Vec<UserPluginInstall>>> {
    let installs = plugins::get_user_plugins(&state.db.pg, auth.user_id).await?;
    Ok(Json(installs))
}

/// POST /api/v1/users/@me/plugins — Install a plugin.
#[derive(serde::Deserialize)]
struct InstallPluginBody {
    plugin_id: Uuid,
}

async fn install_plugin(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<InstallPluginBody>,
) -> NexusResult<axum::http::StatusCode> {
    // Verify plugin exists
    plugins::get_plugin_by_id(&state.db.pg, body.plugin_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "plugin".to_string() })?;

    plugins::install_plugin(&state.db.pg, auth.user_id, body.plugin_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/users/@me/plugins/{plugin_id}
async fn uninstall_plugin(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(plugin_id): Path<Uuid>,
) -> NexusResult<axum::http::StatusCode> {
    plugins::uninstall_plugin(&state.db.pg, auth.user_id, plugin_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// PATCH /api/v1/users/@me/plugins/{plugin_id}
async fn update_plugin_settings(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(plugin_id): Path<Uuid>,
    Json(body): Json<UpdatePluginSettingsRequest>,
) -> NexusResult<axum::http::StatusCode> {
    plugins::update_plugin_install(
        &state.db.pg,
        auth.user_id,
        plugin_id,
        body.enabled,
        body.settings,
    )
    .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ============================================================================
// Theme Marketplace
// ============================================================================

/// GET /api/v1/themes
async fn list_themes(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(q): Query<PaginationQuery>,
) -> NexusResult<Json<Vec<Theme>>> {
    let limit = q.limit.min(100);
    let items = plugins::list_themes(&state.db.pg, limit, q.offset).await?;
    Ok(Json(items))
}

/// GET /api/v1/themes/{slug}
async fn get_theme(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> NexusResult<Json<Theme>> {
    let theme = plugins::get_theme_by_slug(&state.db.pg, &slug)
        .await?
        .ok_or(NexusError::NotFound { resource: "theme".to_string() })?;
    Ok(Json(theme))
}

/// POST /api/v1/themes — Submit a new theme (pending review).
async fn submit_theme(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<SubmitThemeRequest>,
) -> NexusResult<Json<Theme>> {
    let id = snowflake::generate_id();
    let theme = plugins::create_theme(
        &state.db.pg,
        id,
        Some(auth.user_id),
        &body.name,
        &body.slug,
        body.description.as_deref(),
        &body.version,
        body.preview_url.as_deref(),
        &body.css,
        &body.variables,
    )
    .await?;
    Ok(Json(theme))
}

// ============================================================================
// User Theme Installs
// ============================================================================

/// GET /api/v1/users/@me/themes
async fn get_my_themes(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<Vec<UserThemeInstall>>> {
    let installs = plugins::get_user_themes(&state.db.pg, auth.user_id).await?;
    Ok(Json(installs))
}

/// POST /api/v1/users/@me/themes — Install a theme.
#[derive(serde::Deserialize)]
struct InstallThemeBody {
    theme_id: Uuid,
}

async fn install_theme(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<InstallThemeBody>,
) -> NexusResult<axum::http::StatusCode> {
    plugins::install_theme(&state.db.pg, auth.user_id, body.theme_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/users/@me/themes/{theme_id}
async fn uninstall_theme(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(theme_id): Path<Uuid>,
) -> NexusResult<axum::http::StatusCode> {
    plugins::uninstall_theme(&state.db.pg, auth.user_id, theme_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// POST /api/v1/users/@me/themes/{theme_id}/activate — Switch active theme.
async fn activate_theme(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(theme_id): Path<Uuid>,
) -> NexusResult<axum::http::StatusCode> {
    let success = plugins::activate_theme(&state.db.pg, auth.user_id, theme_id).await?;
    if !success {
        return Err(NexusError::NotFound { resource: "theme".to_string() });
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}
