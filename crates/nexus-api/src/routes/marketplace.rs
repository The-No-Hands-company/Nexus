//! Plugin marketplace routes — Phase 19-04.
//!
//! GET    /marketplace/plugins                     — Browse / search plugins
//! GET    /marketplace/plugins/:slug               — Get plugin by slug
//! POST   /marketplace/plugins                     — Publish a new plugin
//! POST   /marketplace/plugins/:id/reviews         — Submit / update review
//! GET    /marketplace/plugins/:id/reviews         — List reviews for a plugin
//! DELETE /marketplace/plugins/:id/reviews         — Delete own review
//! POST   /servers/:server_id/plugin-installs      — Install a plugin
//! GET    /servers/:server_id/plugin-installs      — List installed plugins
//! PATCH  /servers/:server_id/plugin-installs/:plugin_id — Toggle enable/disable
//! DELETE /servers/:server_id/plugin-installs/:plugin_id — Uninstall

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::models::ecosystem::{MarketplacePlugin, PluginInstall, PluginReview, ReviewStatus, TrustTier};
use nexus_db::repository::{marketplace, members};
use nexus_common::models::Member;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Marketplace browse / publish
        .route(
            "/marketplace/plugins",
            get(search_plugins).post(publish_plugin),
        )
        .route("/marketplace/plugins/:slug", get(get_plugin_by_slug))
        // Reviews (user ratings)
        .route(
            "/marketplace/plugins/:plugin_id/reviews",
            get(list_reviews).post(submit_review).delete(delete_review),
        )
        // Store governance (admin endpoints)
        .route(
            "/marketplace/plugins/:plugin_id/submit-review",
            post(submit_plugin_for_review),
        )
        .route(
            "/marketplace/admin/review-queue",
            get(get_review_queue),
        )
        .route(
            "/marketplace/admin/plugins/:plugin_id/approve",
            post(approve_plugin),
        )
        .route(
            "/marketplace/admin/plugins/:plugin_id/reject",
            post(reject_plugin),
        )
        .route(
            "/marketplace/admin/plugins/:plugin_id/quarantine",
            post(quarantine_plugin),
        )
        .route(
            "/marketplace/admin/plugins/:plugin_id/takedown",
            post(request_takedown_admin),
        )
        .route(
            "/marketplace/admin/takedowns/:takedown_id/review",
            post(review_takedown),
        )
        .route(
            "/marketplace/admin/takedowns/:takedown_id/reinstate",
            post(reinstate_plugin_admin),
        )
        // Server installs
        .route(
            "/servers/:server_id/plugin-installs",
            post(install_plugin).get(list_installs),
        )
        .route(
            "/servers/:server_id/plugin-installs/:plugin_id",
            axum::routing::patch(toggle_install).delete(uninstall_plugin),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request / Query Types ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: Option<String>,
    category: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PublishPluginRequest {
    name: String,
    slug: String,
    description: Option<String>,
    version: String,
    manifest_url: String,
    icon_url: Option<String>,
    source_url: Option<String>,
    signature: Option<String>,
    signing_key_id: Option<String>,
    category: String,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct SubmitReviewRequest {
    rating: i16,
    title: Option<String>,
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InstallRequest {
    plugin_id: Uuid,
    version: String,
}

#[derive(Debug, Deserialize)]
struct ToggleRequest {
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct ApprovePluginRequest {
    trust_tier: Option<String>, // "reviewed" or "verified", default "reviewed"
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RejectPluginRequest {
    reason: String,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QuarantinePluginRequest {
    reason: String,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TakedownRequest {
    reason: String, // 'copyright', 'malware', 'abuse', 'spam', 'tos_violation'
    description: String,
    evidence_urls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ReviewTakedownRequest {
    status: String, // 'pending', 'quarantined', 'reviewed', 'reinstated', 'permanent_takedown'
    notes: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn search_plugins(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SearchQuery>,
) -> NexusResult<Json<Vec<MarketplacePlugin>>> {
    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);

    let plugins = marketplace::search_plugins(
        &state.db.pool,
        q.q.as_deref(),
        q.category.as_deref(),
        limit,
        offset,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(plugins))
}

async fn get_plugin_by_slug(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> NexusResult<Json<MarketplacePlugin>> {
    let plugin = marketplace::get_plugin_by_slug(&state.db.pool, &slug)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or_else(|| NexusError::NotFound {
            resource: "marketplace_plugin".into(),
        })?;

    Ok(Json(plugin))
}

async fn publish_plugin(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<PublishPluginRequest>,
) -> NexusResult<Json<MarketplacePlugin>> {
    if body.name.is_empty() || body.slug.is_empty() {
        return Err(NexusError::Validation {
            message: "name and slug are required".into(),
        });
    }

    // Validate slug format (lowercase alphanumeric + hyphens)
    if !body
        .slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(NexusError::Validation {
            message: "slug must contain only lowercase letters, digits, and hyphens".into(),
        });
    }

    let id = Uuid::new_v4();
    let tags = serde_json::to_value(body.tags.unwrap_or_default())
        .unwrap_or(serde_json::Value::Array(vec![]));

    let plugin = marketplace::create_plugin(
        &state.db.pool,
        id,
        &body.name,
        &body.slug,
        body.description.as_deref(),
        Some(ctx.user_id),
        &body.version,
        &body.manifest_url,
        body.icon_url.as_deref(),
        body.source_url.as_deref(),
        body.signature.as_deref(),
        body.signing_key_id.as_deref(),
        &body.category,
        &tags,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(plugin))
}

// ── Reviews ───────────────────────────────────────────────────────────────────

async fn submit_review(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(plugin_id): Path<Uuid>,
    Json(body): Json<SubmitReviewRequest>,
) -> NexusResult<Json<PluginReview>> {
    if !(1..=5).contains(&body.rating) {
        return Err(NexusError::Validation {
            message: "rating must be between 1 and 5".into(),
        });
    }

    let id = Uuid::new_v4();
    let review = marketplace::create_review(
        &state.db.pool,
        id,
        plugin_id,
        ctx.user_id,
        body.rating,
        body.title.as_deref(),
        body.body.as_deref(),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    // Update plugin's avg rating
    let _ = marketplace::update_plugin_rating(&state.db.pool, plugin_id).await;

    Ok(Json(review))
}

async fn list_reviews(
    State(state): State<Arc<AppState>>,
    Path(plugin_id): Path<Uuid>,
) -> NexusResult<Json<Vec<PluginReview>>> {
    let reviews = marketplace::list_reviews(&state.db.pool, plugin_id, 100)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(reviews))
}

async fn delete_review(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(plugin_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let deleted = marketplace::delete_review(&state.db.pool, plugin_id, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    if !deleted {
        return Err(NexusError::NotFound {
            resource: "plugin_review".into(),
        });
    }

    // Update avg rating
    let _ = marketplace::update_plugin_rating(&state.db.pool, plugin_id).await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ── Server Installs ───────────────────────────────────────────────────────────

async fn install_plugin(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<InstallRequest>,
) -> NexusResult<Json<PluginInstall>> {
    let _member: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _member.is_none() { return Err(NexusError::Forbidden); }

    // Verify plugin exists
    let _plugin = marketplace::get_plugin(&state.db.pool, body.plugin_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or_else(|| NexusError::NotFound {
            resource: "marketplace_plugin".into(),
        })?;

    let id = Uuid::new_v4();
    let install = marketplace::install_plugin(
        &state.db.pool,
        id,
        body.plugin_id,
        server_id,
        ctx.user_id,
        &body.version,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    // Increment download counter
    let _ = marketplace::increment_downloads(&state.db.pool, body.plugin_id).await;

    Ok(Json(install))
}

async fn list_installs(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<PluginInstall>>> {
    let _member: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _member.is_none() { return Err(NexusError::Forbidden); }

    let installs = marketplace::list_server_installs(&state.db.pool, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(installs))
}

async fn toggle_install(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path((server_id, plugin_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ToggleRequest>,
) -> NexusResult<Json<PluginInstall>> {
    let _member: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _member.is_none() { return Err(NexusError::Forbidden); }

    let install =
        marketplace::toggle_plugin_install(&state.db.pool, plugin_id, server_id, body.enabled)
            .await
            .map_err(|e| NexusError::Internal(e.into()))?
            .ok_or_else(|| NexusError::NotFound {
                resource: "plugin_install".into(),
            })?;

    Ok(Json(install))
}

async fn uninstall_plugin(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path((server_id, plugin_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let _member: Option<Member> = members::find_member(&state.db.pool, ctx.user_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if _member.is_none() { return Err(NexusError::Forbidden); }

    let deleted = marketplace::uninstall_plugin(&state.db.pool, plugin_id, server_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    if !deleted {
        return Err(NexusError::NotFound {
            resource: "plugin_install".into(),
        });
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ── Store Governance Handlers ─────────────────────────────────────────────────────

async fn submit_plugin_for_review(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(plugin_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    // Verify user is the plugin author
    let plugin = marketplace::get_plugin(&state.db.pool, plugin_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or_else(|| NexusError::NotFound {
            resource: "marketplace_plugin".into(),
        })?;

    if plugin.author_id != Some(ctx.user_id) {
        return Err(NexusError::Forbidden);
    }

    marketplace::submit_for_review(&state.db.pool, plugin_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(serde_json::json!({ "submitted": true })))
}

async fn get_review_queue(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Query(limit): Query<Option<i64>>,
) -> NexusResult<Json<serde_json::Value>> {
    // TODO: Check if user is admin/moderator
    
    let items = marketplace::get_review_queue(&state.db.pool, limit.unwrap_or(20), 0)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(serde_json::json!({
        "queue": items,
        "count": items.len()
    })))
}

async fn approve_plugin(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(plugin_id): Path<Uuid>,
    Json(body): Json<ApprovePluginRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    // TODO: Check if user is admin/moderator
    
    let tier = match body.trust_tier.as_deref().unwrap_or("reviewed") {
        "verified" => TrustTier::Verified,
        _ => TrustTier::Reviewed,
    };

    marketplace::update_review_status(
        &state.db.pool,
        plugin_id,
        ReviewStatus::Approved,
        ctx.user_id,
        None,
        body.notes.as_deref(),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    marketplace::update_trust_tier(&state.db.pool, plugin_id, tier)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    marketplace::log_review(
        &state.db.pool,
        Uuid::new_v4(),
        plugin_id,
        Some(ctx.user_id),
        "review",
        "approved",
        "approved",
        None,
        body.notes.as_deref(),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(serde_json::json!({ "approved": true })))
}

async fn reject_plugin(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(plugin_id): Path<Uuid>,
    Json(body): Json<RejectPluginRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    // TODO: Check if user is admin/moderator
    
    marketplace::update_review_status(
        &state.db.pool,
        plugin_id,
        ReviewStatus::Rejected,
        ctx.user_id,
        Some(&body.reason),
        body.notes.as_deref(),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    marketplace::log_review(
        &state.db.pool,
        Uuid::new_v4(),
        plugin_id,
        Some(ctx.user_id),
        "review",
        "rejected",
        "rejected",
        Some(&body.reason),
        body.notes.as_deref(),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(serde_json::json!({ "rejected": true })))
}

async fn quarantine_plugin(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(plugin_id): Path<Uuid>,
    Json(body): Json<QuarantinePluginRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    // TODO: Check if user is admin/moderator
    
    marketplace::update_review_status(
        &state.db.pool,
        plugin_id,
        ReviewStatus::Quarantined,
        ctx.user_id,
        Some(&body.reason),
        body.notes.as_deref(),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    marketplace::log_review(
        &state.db.pool,
        Uuid::new_v4(),
        plugin_id,
        Some(ctx.user_id),
        "review",
        "quarantined",
        "quarantined",
        Some(&body.reason),
        body.notes.as_deref(),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(serde_json::json!({ "quarantined": true })))
}

async fn request_takedown_admin(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(plugin_id): Path<Uuid>,
    Json(body): Json<TakedownRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    // TODO: Check if user is admin/moderator
    
    let takedown_id = Uuid::new_v4();
    marketplace::request_takedown(
        &state.db.pool,
        takedown_id,
        plugin_id,
        Some(ctx.user_id),
        &body.reason,
        &body.description,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(serde_json::json!({
        "takedown_id": takedown_id,
        "plugin_id": plugin_id
    })))
}

async fn review_takedown(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(takedown_id): Path<Uuid>,
    Json(body): Json<ReviewTakedownRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    // TODO: Check if user is admin/moderator
    
    marketplace::review_takedown(
        &state.db.pool,
        takedown_id,
        ctx.user_id,
        &body.status,
        body.notes.as_deref(),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(serde_json::json!({ "reviewed": true })))
}

async fn reinstate_plugin_admin(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(takedown_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    // TODO: Check if user is admin/moderator
    
    marketplace::reinstate_plugin(&state.db.pool, takedown_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(serde_json::json!({ "reinstated": true })))
}
}
