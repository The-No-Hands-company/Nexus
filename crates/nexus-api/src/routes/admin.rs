//! Instance admin routes — user management + overview stats.
//!
//! All routes require the `INSTANCE_ADMIN` user flag (bit 7 of users.flags).
//! Grant it with: UPDATE users SET flags = flags | 128 WHERE username = 'you';
//!
//! Endpoints:
//!   GET  /admin/overview            — aggregate instance stats
//!   GET  /admin/users               — paginated user list (search, filter by flag)
//!   GET  /admin/users/{id}          — single user detail
//!   PATCH /admin/users/{id}         — update flags (suspend, disable, grant admin)
//!   GET  /admin/servers             — all servers on this instance
//!   DELETE /admin/servers/{id}      — delete a server (destructive)

use axum::{
    extract::{Path, Query, State},
    middleware,
    routing::{delete, get, patch},
    Json, Router,
};
use nexus_common::{
    error::{NexusError, NexusResult},
    models::user::user_flags,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};
use axum::extract::Extension;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/admin/overview", get(get_overview))
        .route("/admin/users/stats", get(user_stats_handler))
        .route("/admin/users", get(list_users))
        .route("/admin/users/:id", get(get_user).patch(update_user))
        .route("/admin/users/:id/suspend", axum::routing::post(suspend_user_handler))
        .route("/admin/users/:id/unsuspend", axum::routing::post(unsuspend_user_handler))
        .route("/admin/users/:id/disable", axum::routing::post(disable_user_handler))
        .route("/admin/servers", get(list_all_servers))
        .route("/admin/servers/:id", delete(delete_server))
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

// ── Auth guard ────────────────────────────────────────────────────────────────

async fn require_instance_admin(pool: &sqlx::AnyPool, user_id: Uuid) -> NexusResult<()> {
    let row = sqlx::query("SELECT flags FROM users WHERE id = $1::uuid")
        .bind(user_id.to_string())
        .fetch_one(pool)
        .await
        .map_err(|_| NexusError::Forbidden)?;
    let flags: i64 = row.try_get("flags").unwrap_or(0);
    if flags & user_flags::INSTANCE_ADMIN == 0 {
        return Err(NexusError::Forbidden);
    }
    Ok(())
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct InstanceOverview {
    total_users: i64,
    active_users_24h: i64,
    suspended_users: i64,
    total_servers: i64,
    total_messages: i64,
    /// Voice users currently connected
    voice_connections: usize,
    /// Seconds since server start
    uptime_secs: u64,
    /// Server version
    version: &'static str,
    /// Gateway status summary
    gateway_online: bool,
}

#[derive(Serialize)]
struct AdminUser {
    id: String,
    username: String,
    display_name: Option<String>,
    email: Option<String>,
    flags: i64,
    presence: String,
    created_at: String,
    is_remote: bool,
    /// Human-readable flag summary
    flag_labels: Vec<&'static str>,
}

#[derive(Deserialize)]
struct ListUsersParams {
    /// Full-text search on username / display_name
    q: Option<String>,
    /// Filter by a specific flag bit (e.g. ?flag=128 for admins)
    flag: Option<i64>,
    /// Return users where flag bit is NOT set (e.g. suspended only)
    not_flag: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
struct UserList {
    users: Vec<AdminUser>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Deserialize)]
struct UpdateUserRequest {
    /// Set specific flags on (OR in)
    set_flags: Option<i64>,
    /// Clear specific flags (AND NOT)
    clear_flags: Option<i64>,
}

#[derive(Serialize)]
struct AdminServer {
    id: String,
    name: String,
    description: Option<String>,
    owner_id: String,
    member_count: i64,
    created_at: String,
    is_public: bool,
    boost_tier: i64,
}

#[derive(Serialize)]
struct ServerList {
    servers: Vec<AdminServer>,
    total: i64,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /admin/overview
async fn get_overview(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<InstanceOverview>> {
    require_instance_admin(&state.db.pool, auth.user_id).await?;

    let pool = &state.db.pool;

    // Aggregate counts in parallel
    let total_users: i64 = sqlx::query("SELECT COUNT(*) AS c FROM users WHERE is_remote = FALSE")
        .fetch_one(pool)
        .await
        .map(|r| r.try_get::<i64, _>("c").unwrap_or(0))
        .unwrap_or(0);

    let active_users_24h: i64 = sqlx::query(
        "SELECT COUNT(*) AS c FROM users \
         WHERE is_remote = FALSE AND updated_at > NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(pool)
    .await
    .map(|r| r.try_get::<i64, _>("c").unwrap_or(0))
    .unwrap_or(0);

    let suspended_users: i64 = sqlx::query(
        &format!("SELECT COUNT(*) AS c FROM users WHERE flags & {} != 0", user_flags::SUSPENDED),
    )
    .fetch_one(pool)
    .await
    .map(|r| r.try_get::<i64, _>("c").unwrap_or(0))
    .unwrap_or(0);

    let total_servers: i64 = sqlx::query("SELECT COUNT(*) AS c FROM servers")
        .fetch_one(pool)
        .await
        .map(|r| r.try_get::<i64, _>("c").unwrap_or(0))
        .unwrap_or(0);

    // Messages is an approximation — exact count is expensive on large instances
    let total_messages: i64 = sqlx::query("SELECT COUNT(*) AS c FROM messages")
        .fetch_one(pool)
        .await
        .map(|r| r.try_get::<i64, _>("c").unwrap_or(0))
        .unwrap_or(0);

    let voice_connections = state.voice_state.stats().await.total_connections;

    let uptime_secs = state.started_at.elapsed().as_secs();

    Ok(Json(InstanceOverview {
        total_users,
        active_users_24h,
        suspended_users,
        total_servers,
        total_messages,
        voice_connections,
        uptime_secs,
        version: env!("CARGO_PKG_VERSION"),
        gateway_online: true,
    }))
}

/// GET /admin/users
async fn list_users(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListUsersParams>,
) -> NexusResult<Json<UserList>> {
    require_instance_admin(&state.db.pool, auth.user_id).await?;

    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let offset = params.offset.unwrap_or(0).max(0);

    // Build dynamic query — safe because we're only interpolating numeric literals
    // (the search string goes through bind params)
    let mut conditions = vec!["is_remote = FALSE"];
    if let Some(flag) = params.flag {
        // flag condition is a numeric literal from a trusted source (admin query param)
        conditions.push(Box::leak(format!("(flags & {flag}) != 0").into_boxed_str()));
    }
    if let Some(not_flag) = params.not_flag {
        conditions.push(Box::leak(format!("(flags & {not_flag}) = 0").into_boxed_str()));
    }

    let where_clause = conditions.join(" AND ");
    let search_clause = if params.q.is_some() {
        "AND (username ILIKE $3 OR display_name ILIKE $3)"
    } else {
        ""
    };

    let count_q = format!(
        "SELECT COUNT(*) AS c FROM users WHERE {where_clause} {search_clause}"
    );
    let data_q = format!(
        "SELECT id::text, username, display_name, email, flags, presence::text, \
                created_at::text, is_remote \
         FROM users \
         WHERE {where_clause} {search_clause} \
         ORDER BY created_at DESC \
         LIMIT $1 OFFSET $2"
    );

    let search_pattern = params.q.as_ref().map(|q| format!("%{q}%"));

    // Count
    let total: i64 = if let Some(ref pat) = search_pattern {
        sqlx::query(&count_q)
            .bind(pat)
            .fetch_one(&state.db.pool)
            .await
            .map(|r| r.try_get::<i64, _>("c").unwrap_or(0))
            .unwrap_or(0)
    } else {
        sqlx::query(&count_q)
            .fetch_one(&state.db.pool)
            .await
            .map(|r| r.try_get::<i64, _>("c").unwrap_or(0))
            .unwrap_or(0)
    };

    // Data
    let rows = if let Some(ref pat) = search_pattern {
        sqlx::query(&data_q)
            .bind(limit)
            .bind(offset)
            .bind(pat)
            .fetch_all(&state.db.pool)
            .await
    } else {
        sqlx::query(&data_q)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.db.pool)
            .await
    }
    .map_err(|e| NexusError::Internal(e.into()))?;

    let users = rows
        .iter()
        .map(|r| {
            let flags: i64 = r.try_get("flags").unwrap_or(0);
            AdminUser {
                id: r.try_get("id").unwrap_or_default(),
                username: r.try_get("username").unwrap_or_default(),
                display_name: r.try_get("display_name").ok().flatten(),
                email: r.try_get("email").ok().flatten(),
                flags,
                presence: r.try_get("presence").unwrap_or_else(|_| "offline".to_string()),
                created_at: r.try_get("created_at").unwrap_or_default(),
                is_remote: r.try_get("is_remote").unwrap_or(false),
                flag_labels: flag_labels(flags),
            }
        })
        .collect();

    Ok(Json(UserList { users, total, limit, offset }))
}

/// GET /admin/users/:id
async fn get_user(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> NexusResult<Json<AdminUser>> {
    require_instance_admin(&state.db.pool, auth.user_id).await?;

    let row = sqlx::query(
        "SELECT id::text, username, display_name, email, flags, presence::text, \
                created_at::text, is_remote \
         FROM users WHERE id = $1::uuid",
    )
    .bind(user_id.to_string())
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?
    .ok_or(NexusError::NotFound { resource: "User".into() })?;

    let flags: i64 = row.try_get("flags").unwrap_or(0);
    Ok(Json(AdminUser {
        id: row.try_get("id").unwrap_or_default(),
        username: row.try_get("username").unwrap_or_default(),
        display_name: row.try_get("display_name").ok().flatten(),
        email: row.try_get("email").ok().flatten(),
        flags,
        presence: row.try_get("presence").unwrap_or_else(|_| "offline".to_string()),
        created_at: row.try_get("created_at").unwrap_or_default(),
        is_remote: row.try_get("is_remote").unwrap_or(false),
        flag_labels: flag_labels(flags),
    }))
}

/// PATCH /admin/users/:id — set or clear flag bits
async fn update_user(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UpdateUserRequest>,
) -> NexusResult<Json<AdminUser>> {
    require_instance_admin(&state.db.pool, auth.user_id).await?;

    // Build the SET clause based on which flags to set / clear
    match (body.set_flags, body.clear_flags) {
        (None, None) => {
            return Err(NexusError::Validation {
                message: "Provide set_flags or clear_flags".into(),
            })
        }
        (Some(set), None) => {
            sqlx::query("UPDATE users SET flags = flags | $1, updated_at = NOW() WHERE id = $2::uuid")
                .bind(set)
                .bind(user_id.to_string())
                .execute(&state.db.pool)
                .await
                .map_err(|e| NexusError::Internal(e.into()))?;
        }
        (None, Some(clear)) => {
            sqlx::query("UPDATE users SET flags = flags & ~$1, updated_at = NOW() WHERE id = $2::uuid")
                .bind(clear)
                .bind(user_id.to_string())
                .execute(&state.db.pool)
                .await
                .map_err(|e| NexusError::Internal(e.into()))?;
        }
        (Some(set), Some(clear)) => {
            // Apply both: clear first, then set
            sqlx::query(
                "UPDATE users SET flags = (flags & ~$1) | $2, updated_at = NOW() WHERE id = $3::uuid",
            )
            .bind(clear)
            .bind(set)
            .bind(user_id.to_string())
            .execute(&state.db.pool)
            .await
            .map_err(|e| NexusError::Internal(e.into()))?;
        }
    }

    tracing::info!(
        admin = %auth.user_id,
        target_user = %user_id,
        set_flags = ?body.set_flags,
        clear_flags = ?body.clear_flags,
        "Admin updated user flags"
    );

    // Return updated user
    get_user(Extension(auth), State(state), Path(user_id)).await
}

/// GET /admin/servers
async fn list_all_servers(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<ServerList>> {
    require_instance_admin(&state.db.pool, auth.user_id).await?;

    let rows = sqlx::query(
        "SELECT id::text, name, description, owner_id::text, member_count, \
                created_at::text, is_public, boost_tier \
         FROM servers \
         ORDER BY member_count DESC, created_at DESC \
         LIMIT 1000",
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    let total = rows.len() as i64;
    let servers = rows
        .iter()
        .map(|r| AdminServer {
            id: r.try_get("id").unwrap_or_default(),
            name: r.try_get("name").unwrap_or_default(),
            description: r.try_get("description").ok().flatten(),
            owner_id: r.try_get("owner_id").unwrap_or_default(),
            member_count: r.try_get("member_count").unwrap_or(0),
            created_at: r.try_get("created_at").unwrap_or_default(),
            is_public: r.try_get("is_public").unwrap_or(false),
            boost_tier: r.try_get("boost_tier").unwrap_or(0),
        })
        .collect();

    Ok(Json(ServerList { servers, total }))
}

/// DELETE /admin/servers/:id — permanently delete a server
async fn delete_server(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    require_instance_admin(&state.db.pool, auth.user_id).await?;

    let affected = sqlx::query("DELETE FROM servers WHERE id = $1::uuid")
        .bind(server_id.to_string())
        .execute(&state.db.pool)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .rows_affected();

    if affected == 0 {
        return Err(NexusError::NotFound { resource: "Server".into() });
    }

    tracing::warn!(
        admin = %auth.user_id,
        server_id = %server_id,
        "Admin deleted server"
    );

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn flag_labels(flags: i64) -> Vec<&'static str> {
    use nexus_common::models::user::user_flags;
    let mut labels = vec![];
    if flags & user_flags::STAFF != 0            { labels.push("staff"); }
    if flags & user_flags::BOT != 0              { labels.push("bot"); }
    if flags & user_flags::EMAIL_VERIFIED != 0   { labels.push("email_verified"); }
    if flags & user_flags::EARLY_SUPPORTER != 0  { labels.push("early_supporter"); }
    if flags & user_flags::PREMIUM_SUPPORTER != 0 { labels.push("premium"); }
    if flags & user_flags::DISABLED != 0         { labels.push("disabled"); }
    if flags & user_flags::SUSPENDED != 0        { labels.push("suspended"); }
    if flags & user_flags::INSTANCE_ADMIN != 0   { labels.push("instance_admin"); }
    labels
}

// ── Additional endpoints referenced by the dashboard ─────────────────────────

/// POST /admin/users/:id/suspend
pub async fn suspend_user_handler(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    require_instance_admin(&state.db.pool, auth.user_id).await?;
    sqlx::query(
        &format!("UPDATE users SET flags = flags | {}, updated_at = NOW() WHERE id = $1::uuid",
                 user_flags::SUSPENDED))
        .bind(user_id.to_string())
        .execute(&state.db.pool).await
        .map_err(|e| NexusError::Internal(e.into()))?;
    tracing::info!(admin = %auth.user_id, target = %user_id, "suspended user");
    Ok(Json(serde_json::json!({ "suspended": true })))
}

/// POST /admin/users/:id/unsuspend
pub async fn unsuspend_user_handler(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    require_instance_admin(&state.db.pool, auth.user_id).await?;
    sqlx::query(
        &format!("UPDATE users SET flags = flags & ~{}, updated_at = NOW() WHERE id = $1::uuid",
                 user_flags::SUSPENDED))
        .bind(user_id.to_string())
        .execute(&state.db.pool).await
        .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(serde_json::json!({ "suspended": false })))
}

/// POST /admin/users/:id/disable
pub async fn disable_user_handler(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    require_instance_admin(&state.db.pool, auth.user_id).await?;
    sqlx::query(
        &format!("UPDATE users SET flags = flags | {}, updated_at = NOW() WHERE id = $1::uuid",
                 user_flags::DISABLED))
        .bind(user_id.to_string())
        .execute(&state.db.pool).await
        .map_err(|e| NexusError::Internal(e.into()))?;
    tracing::info!(admin = %auth.user_id, target = %user_id, "disabled user account");
    Ok(Json(serde_json::json!({ "disabled": true })))
}

/// GET /admin/users/stats — aggregate user stats for the overview
pub async fn user_stats_handler(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<serde_json::Value>> {
    require_instance_admin(&state.db.pool, auth.user_id).await?;
    let pool = &state.db.pool;

    let total: i64 = sqlx::query("SELECT COUNT(*) AS c FROM users WHERE is_remote = FALSE")
        .fetch_one(pool).await.map(|r| r.try_get("c").unwrap_or(0)).unwrap_or(0);
    let suspended: i64 = sqlx::query(
        &format!("SELECT COUNT(*) AS c FROM users WHERE flags & {} != 0", user_flags::SUSPENDED))
        .fetch_one(pool).await.map(|r| r.try_get("c").unwrap_or(0)).unwrap_or(0);
    let disabled: i64 = sqlx::query(
        &format!("SELECT COUNT(*) AS c FROM users WHERE flags & {} != 0", user_flags::DISABLED))
        .fetch_one(pool).await.map(|r| r.try_get("c").unwrap_or(0)).unwrap_or(0);
    let bots: i64 = sqlx::query(
        &format!("SELECT COUNT(*) AS c FROM users WHERE flags & {} != 0", user_flags::BOT))
        .fetch_one(pool).await.map(|r| r.try_get("c").unwrap_or(0)).unwrap_or(0);
    let new_24h: i64 = sqlx::query(
        "SELECT COUNT(*) AS c FROM users WHERE is_remote = FALSE AND created_at > NOW() - INTERVAL '24 hours'")
        .fetch_one(pool).await.map(|r| r.try_get("c").unwrap_or(0)).unwrap_or(0);
    let new_7d: i64 = sqlx::query(
        "SELECT COUNT(*) AS c FROM users WHERE is_remote = FALSE AND created_at > NOW() - INTERVAL '7 days'")
        .fetch_one(pool).await.map(|r| r.try_get("c").unwrap_or(0)).unwrap_or(0);

    Ok(Json(serde_json::json!({
        "total": total, "suspended": suspended, "disabled": disabled,
        "bots": bots, "new_24h": new_24h, "new_7d": new_7d,
    })))
}
