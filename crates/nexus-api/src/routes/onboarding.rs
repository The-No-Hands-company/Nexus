//! Onboarding & server template routes — Phase 19-02.
//!
//! GET    /users/@me/onboarding        — Get onboarding progress
//! PATCH  /users/@me/onboarding        — Update onboarding progress
//! GET    /server-templates            — List available templates
//! GET    /server-templates/:id        — Get a single template
//! POST   /server-templates            — Create a template (admin)
//! DELETE /server-templates/:id        — Delete a template (admin)

use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    middleware,
    routing::get,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::models::ecosystem::{OnboardingProgress, ServerTemplate};
use nexus_db::repository::{onboarding, server_templates};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{AppState, middleware::AuthContext};

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/users/@me/onboarding",
            get(get_onboarding).patch(update_onboarding),
        )
        .route(
            "/server-templates",
            get(list_templates).post(create_template),
        )
        .route(
            "/server-templates/{template_id}",
            get(get_template).delete(delete_template),
        )
        .route_layer(middleware::from_fn(
            crate::middleware::combined_auth_middleware,
        ))
}

// ── Request Bodies ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct UpdateOnboardingRequest {
    completed_steps: Option<serde_json::Value>,
    dismissed: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct TemplateQuery {
    category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateTemplateRequest {
    name: String,
    description: Option<String>,
    category: String,
    icon_url: Option<String>,
    channels: serde_json::Value,
    roles: serde_json::Value,
    settings: Option<serde_json::Value>,
    is_builtin: Option<bool>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn get_onboarding(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<serde_json::Value>> {
    let progress = onboarding::get_progress(&state.db.pool, ctx.user_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    match progress {
        Some(p) => Ok(Json(serde_json::to_value(p).unwrap_or_default())),
        None => Ok(Json(serde_json::json!({
            "user_id": ctx.user_id.to_string(),
            "completed_steps": [],
            "dismissed": false
        }))),
    }
}

async fn update_onboarding(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<UpdateOnboardingRequest>,
) -> NexusResult<Json<OnboardingProgress>> {
    let steps = body
        .completed_steps
        .unwrap_or(serde_json::Value::Array(vec![]));
    let dismissed = body.dismissed.unwrap_or(false);

    let progress = onboarding::upsert_progress(&state.db.pool, ctx.user_id, &steps, dismissed)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(progress))
}

async fn list_templates(
    State(state): State<Arc<AppState>>,
    Query(q): Query<TemplateQuery>,
) -> NexusResult<Json<Vec<ServerTemplate>>> {
    let valid_categories = ["gaming", "education", "teams", "open-source", "community"];
    if let Some(ref cat) = q.category
        && !valid_categories.contains(&cat.as_str()) {
            return Err(NexusError::Validation {
                message: format!("category must be one of: {}", valid_categories.join(", ")),
            });
        }

    let templates = server_templates::list_templates(&state.db.pool, q.category.as_deref(), 100)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(templates))
}

async fn get_template(
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<Uuid>,
) -> NexusResult<Json<ServerTemplate>> {
    let t = server_templates::get_template(&state.db.pool, template_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .ok_or_else(|| NexusError::NotFound {
            resource: "server_template".into(),
        })?;

    Ok(Json(t))
}

async fn create_template(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<CreateTemplateRequest>,
) -> NexusResult<Json<ServerTemplate>> {
    let valid_categories = ["gaming", "education", "teams", "open-source", "community"];
    if !valid_categories.contains(&body.category.as_str()) {
        return Err(NexusError::Validation {
            message: format!("category must be one of: {}", valid_categories.join(", ")),
        });
    }

    let id = Uuid::new_v4();
    let settings = body.settings.unwrap_or(serde_json::json!({}));

    let t = server_templates::create_template(
        &state.db.pool,
        id,
        &body.name,
        body.description.as_deref(),
        &body.category,
        body.icon_url.as_deref(),
        &body.channels,
        &body.roles,
        &settings,
        body.is_builtin.unwrap_or(false),
        Some(ctx.user_id),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(t))
}

async fn delete_template(
    State(state): State<Arc<AppState>>,
    Extension(_ctx): Extension<AuthContext>,
    Path(template_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let deleted = server_templates::delete_template(&state.db.pool, template_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;

    if !deleted {
        return Err(NexusError::NotFound {
            resource: "server_template".into(),
        });
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}
