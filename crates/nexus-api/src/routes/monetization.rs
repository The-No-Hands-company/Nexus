//! Creator Monetization API (v0.16) — tip jars, subscription tiers, analytics.
//!
//! ## Endpoints
//!
//! | Method | Path | Auth | Description |
//! |--------|------|------|-------------|
//! | GET    | `/api/v1/servers/{id}/monetization`         | Bearer | Get monetization config |
//! | PUT    | `/api/v1/servers/{id}/monetization`         | Bearer | Update monetization config |
//! | GET    | `/api/v1/servers/{id}/subscription-tiers`   | None   | List subscription tiers |
//! | POST   | `/api/v1/servers/{id}/subscription-tiers`   | Bearer | Create subscription tier |
//! | PATCH  | `/api/v1/servers/{id}/subscription-tiers/{tier_id}` | Bearer | Update tier |
//! | DELETE | `/api/v1/servers/{id}/subscription-tiers/{tier_id}` | Bearer | Delete tier |
//! | GET    | `/api/v1/servers/{id}/analytics`            | Bearer | Creator analytics dashboard |

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_db::repository::servers;
use serde::{Deserialize, Serialize};
use sqlx::Row as _;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

// ─── Router ───────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    let authed = Router::new()
        .route(
            "/servers/{server_id}/monetization",
            get(get_monetization_config).put(update_monetization_config),
        )
        .route(
            "/servers/{server_id}/subscription-tiers",
            post(create_subscription_tier),
        )
        .route(
            "/servers/{server_id}/subscription-tiers/{tier_id}",
            axum::routing::patch(update_subscription_tier).delete(delete_subscription_tier),
        )
        .route(
            "/servers/{server_id}/analytics",
            get(get_analytics),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware));

    let public = Router::new().route(
        "/servers/{server_id}/subscription-tiers",
        get(list_subscription_tiers),
    );

    authed.merge(public)
}

// ─── Types ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct MonetizationConfig {
    pub server_id: Uuid,
    pub provider: String,
    pub webhook_url: Option<String>,
    pub enabled: bool,
    pub tip_jar_url: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateMonetizationRequest {
    pub provider: Option<String>,
    pub webhook_url: Option<String>,
    pub enabled: Option<bool>,
    pub tip_jar_url: Option<String>,
}

#[derive(Serialize)]
pub struct SubscriptionTier {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i32,
    pub currency: String,
    pub interval: String,
    pub role_id: Option<Uuid>,
    pub position: i32,
    pub is_active: bool,
}

#[derive(Deserialize)]
pub struct CreateTierRequest {
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i32,
    pub currency: Option<String>,
    pub interval: Option<String>,
    pub role_id: Option<Uuid>,
    pub position: Option<i32>,
}

#[derive(Deserialize)]
pub struct UpdateTierRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price_cents: Option<i32>,
    pub currency: Option<String>,
    pub interval: Option<String>,
    pub role_id: Option<Uuid>,
    pub position: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Serialize)]
pub struct AnalyticsEntry {
    pub period_date: String,
    pub messages_count: i32,
    pub active_members: i32,
    pub new_members: i32,
    pub left_members: i32,
    pub voice_minutes: i32,
    pub revenue_cents: i32,
    pub new_subscribers: i32,
    pub churned_subscribers: i32,
    pub tip_count: i32,
    pub tip_total_cents: i32,
}

#[derive(Deserialize)]
pub struct AnalyticsQuery {
    /// Number of days to look back (default 30, max 365)
    pub days: Option<i64>,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Verify the requesting user is the server owner.
async fn require_owner(
    pool: &sqlx::AnyPool,
    server_id: Uuid,
    user_id: Uuid,
) -> NexusResult<()> {
    let server = servers::find_by_id(pool, server_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Server".into() })?;
    if server.owner_id != user_id {
        return Err(NexusError::Forbidden);
    }
    Ok(())
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// `GET /api/v1/servers/:server_id/monetization`
async fn get_monetization_config(
    axum::Extension(auth): axum::Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<MonetizationConfig>> {
    require_owner(&state.db.pool, server_id, auth.user_id).await?;

    let row = sqlx::query(
        "SELECT server_id::text AS server_id, provider, webhook_url, enabled \
         FROM creator_payment_config WHERE server_id = $1::uuid",
    )
    .bind(server_id.to_string())
    .fetch_optional(&state.db.pool)
    .await?;

    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Server".into() })?;

    match row {
        Some(r) => Ok(Json(MonetizationConfig {
            server_id,
            provider: r.try_get("provider").unwrap_or_else(|_| "none".into()),
            webhook_url: r.try_get("webhook_url").ok().flatten(),
            enabled: r.try_get("enabled").unwrap_or(false),
            tip_jar_url: server.tip_jar_url,
        })),
        None => Ok(Json(MonetizationConfig {
            server_id,
            provider: "none".into(),
            webhook_url: None,
            enabled: false,
            tip_jar_url: server.tip_jar_url,
        })),
    }
}

/// `PUT /api/v1/servers/:server_id/monetization`
async fn update_monetization_config(
    axum::Extension(auth): axum::Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<UpdateMonetizationRequest>,
) -> NexusResult<Json<MonetizationConfig>> {
    require_owner(&state.db.pool, server_id, auth.user_id).await?;

    // Upsert the payment config
    sqlx::query(
        "INSERT INTO creator_payment_config (server_id, provider, webhook_url, enabled) \
         VALUES ($1::uuid, COALESCE($2, 'none'), $3, COALESCE($4, false)) \
         ON CONFLICT (server_id) DO UPDATE SET \
             provider = COALESCE($2, creator_payment_config.provider), \
             webhook_url = COALESCE($3, creator_payment_config.webhook_url), \
             enabled = COALESCE($4, creator_payment_config.enabled), \
             updated_at = CURRENT_TIMESTAMP",
    )
    .bind(server_id.to_string())
    .bind(&body.provider)
    .bind(&body.webhook_url)
    .bind(body.enabled)
    .execute(&state.db.pool)
    .await?;

    // Update tip_jar_url on the server itself if provided
    if let Some(ref url) = body.tip_jar_url {
        servers::update_server(
            &state.db.pool,
            server_id,
            None, None, None, None, None, None,
            None, None,
            Some(url.as_str()),
        ).await?;
    }

    // Return current config
    get_monetization_config(
        axum::Extension(auth),
        State(state),
        Path(server_id),
    ).await
}

/// `GET /api/v1/servers/:server_id/subscription-tiers`
async fn list_subscription_tiers(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<SubscriptionTier>>> {
    let rows = sqlx::query(
        "SELECT id::text AS id, server_id::text AS server_id, name, description, \
         price_cents, currency, interval, role_id::text AS role_id, position, is_active \
         FROM creator_subscription_tiers \
         WHERE server_id = $1::uuid AND is_active = true \
         ORDER BY position ASC",
    )
    .bind(server_id.to_string())
    .fetch_all(&state.db.pool)
    .await?;

    let tiers = rows
        .into_iter()
        .map(|r| SubscriptionTier {
            id: r.try_get::<String, _>("id").ok()
                .and_then(|s| Uuid::parse_str(&s).ok())
                .unwrap_or_default(),
            server_id,
            name: r.try_get("name").unwrap_or_default(),
            description: r.try_get("description").ok().flatten(),
            price_cents: r.try_get("price_cents").unwrap_or(0),
            currency: r.try_get("currency").unwrap_or_else(|_| "USD".into()),
            interval: r.try_get("interval").unwrap_or_else(|_| "monthly".into()),
            role_id: r
                .try_get::<String, _>("role_id")
                .ok()
                .and_then(|s| Uuid::parse_str(&s).ok()),
            position: r.try_get("position").unwrap_or(0),
            is_active: r.try_get("is_active").unwrap_or(true),
        })
        .collect();

    Ok(Json(tiers))
}

/// `POST /api/v1/servers/:server_id/subscription-tiers`
async fn create_subscription_tier(
    axum::Extension(auth): axum::Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateTierRequest>,
) -> NexusResult<Json<SubscriptionTier>> {
    require_owner(&state.db.pool, server_id, auth.user_id).await?;

    if body.name.is_empty() || body.name.len() > 64 {
        return Err(NexusError::Validation {
            message: "Tier name must be 1-64 characters".into(),
        });
    }
    if body.price_cents < 0 {
        return Err(NexusError::Validation {
            message: "Price must be non-negative".into(),
        });
    }

    let currency = body.currency.as_deref().unwrap_or("USD");
    let interval = body.interval.as_deref().unwrap_or("monthly");
    let position = body.position.unwrap_or(0);

    let row = sqlx::query(
        "INSERT INTO creator_subscription_tiers \
         (server_id, name, description, price_cents, currency, interval, role_id, position) \
         VALUES ($1::uuid, $2, $3, $4, $5, $6, $7::uuid, $8) \
         RETURNING id::text AS id",
    )
    .bind(server_id.to_string())
    .bind(&body.name)
    .bind(&body.description)
    .bind(body.price_cents)
    .bind(currency)
    .bind(interval)
    .bind(body.role_id.map(|u| u.to_string()))
    .bind(position)
    .fetch_one(&state.db.pool)
    .await?;

    let id: String = row.try_get("id")?;
    Ok(Json(SubscriptionTier {
        id: Uuid::parse_str(&id).unwrap_or_default(),
        server_id,
        name: body.name,
        description: body.description,
        price_cents: body.price_cents,
        currency: currency.into(),
        interval: interval.into(),
        role_id: body.role_id,
        position,
        is_active: true,
    }))
}

/// `PATCH /api/v1/servers/:server_id/subscription-tiers/:tier_id`
async fn update_subscription_tier(
    axum::Extension(auth): axum::Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((server_id, tier_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateTierRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    require_owner(&state.db.pool, server_id, auth.user_id).await?;

    sqlx::query(
        "UPDATE creator_subscription_tiers SET \
             name = COALESCE($1, name), \
             description = COALESCE($2, description), \
             price_cents = COALESCE($3, price_cents), \
             currency = COALESCE($4, currency), \
             interval = COALESCE($5, interval), \
             role_id = COALESCE($6::uuid, role_id), \
             position = COALESCE($7, position), \
             is_active = COALESCE($8, is_active), \
             updated_at = CURRENT_TIMESTAMP \
         WHERE id = $9::uuid AND server_id = $10::uuid",
    )
    .bind(&body.name)
    .bind(&body.description)
    .bind(body.price_cents)
    .bind(&body.currency)
    .bind(&body.interval)
    .bind(body.role_id.map(|u| u.to_string()))
    .bind(body.position)
    .bind(body.is_active)
    .bind(tier_id.to_string())
    .bind(server_id.to_string())
    .execute(&state.db.pool)
    .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// `DELETE /api/v1/servers/:server_id/subscription-tiers/:tier_id`
async fn delete_subscription_tier(
    axum::Extension(auth): axum::Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((server_id, tier_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    require_owner(&state.db.pool, server_id, auth.user_id).await?;

    sqlx::query(
        "DELETE FROM creator_subscription_tiers \
         WHERE id = $1::uuid AND server_id = $2::uuid",
    )
    .bind(tier_id.to_string())
    .bind(server_id.to_string())
    .execute(&state.db.pool)
    .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// `GET /api/v1/servers/:server_id/analytics?days=30`
async fn get_analytics(
    axum::Extension(auth): axum::Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<AnalyticsQuery>,
) -> NexusResult<Json<Vec<AnalyticsEntry>>> {
    require_owner(&state.db.pool, server_id, auth.user_id).await?;

    let days = q.days.unwrap_or(30).min(365);

    let rows = sqlx::query(
        "SELECT period_date::text AS period_date, messages_count, active_members, \
         new_members, left_members, voice_minutes, revenue_cents, \
         new_subscribers, churned_subscribers, tip_count, tip_total_cents \
         FROM creator_analytics \
         WHERE server_id = $1::uuid AND period_date >= CURRENT_DATE - ($2 || ' days')::interval \
         ORDER BY period_date ASC",
    )
    .bind(server_id.to_string())
    .bind(days.to_string())
    .fetch_all(&state.db.pool)
    .await?;

    let entries = rows
        .into_iter()
        .map(|r| AnalyticsEntry {
            period_date: r.try_get("period_date").unwrap_or_default(),
            messages_count: r.try_get("messages_count").unwrap_or(0),
            active_members: r.try_get("active_members").unwrap_or(0),
            new_members: r.try_get("new_members").unwrap_or(0),
            left_members: r.try_get("left_members").unwrap_or(0),
            voice_minutes: r.try_get("voice_minutes").unwrap_or(0),
            revenue_cents: r.try_get("revenue_cents").unwrap_or(0),
            new_subscribers: r.try_get("new_subscribers").unwrap_or(0),
            churned_subscribers: r.try_get("churned_subscribers").unwrap_or(0),
            tip_count: r.try_get("tip_count").unwrap_or(0),
            tip_total_cents: r.try_get("tip_total_cents").unwrap_or(0),
        })
        .collect();

    Ok(Json(entries))
}
