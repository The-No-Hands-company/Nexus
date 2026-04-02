//! Sticker pack routes.
//!
//! GET    /sticker-packs                         — list global packs
//! GET    /servers/{id}/stickers                 — list server stickers
//! POST   /servers/{id}/stickers                 — upload sticker (MANAGE_EMOJIS)
//! PATCH  /servers/{id}/stickers/{sid}           — rename sticker
//! DELETE /servers/{id}/stickers/{sid}           — delete sticker

use axum::{
    extract::{Extension, Multipart, Path, State},
    middleware,
    routing::{get, patch},
    Json, Router,
};
use chrono::{DateTime, Utc};
use nexus_common::{
    error::{NexusError, NexusResult},
    gateway_event::{event_types, GatewayEvent},
    permissions::Permissions,
    snowflake,
};
use nexus_db::repository::{members, roles, servers};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sticker-packs", get(list_global_packs))
        .route(
            "/servers/{server_id}/stickers",
            get(list_server_stickers).post(upload_sticker),
        )
        .route(
            "/servers/{server_id}/stickers/{sticker_id}",
            patch(rename_sticker).delete(delete_sticker),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ─────────────────────────────────────────────────────────────────────────────
// Models
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct StickerPack {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub server_id: Option<Uuid>,
    pub is_premium: bool,
    pub stickers: Vec<Sticker>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sticker {
    pub id: Uuid,
    pub pack_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub asset_url: String,
    #[serde(rename = "type")]
    pub sticker_type: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct RenameRequest {
    name: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// DateTime helper
// ─────────────────────────────────────────────────────────────────────────────

/// Parse a PostgreSQL timestamptz ::text value into DateTime<Utc>.
fn parse_pg_dt(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    let normalized = if s.ends_with("+00") {
        format!("{}00", s)
    } else {
        s.to_string()
    };
    if let Ok(dt) = DateTime::parse_from_str(&normalized, "%Y-%m-%d %H:%M:%S%.f%z") {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = DateTime::parse_from_str(&normalized, "%Y-%m-%d %H:%M:%S%z") {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Permission helper
// ─────────────────────────────────────────────────────────────────────────────

async fn require_manage_emojis(
    state: &AppState,
    user_id: Uuid,
    server_id: Uuid,
) -> NexusResult<()> {
    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Server".into() })?;
    if user_id == server.owner_id {
        return Ok(());
    }
    let member = members::find_member(&state.db.pool, user_id, server_id)
        .await?
        .ok_or(NexusError::Forbidden)?;
    let all_roles = roles::list_server_roles(&state.db.pool, server_id).await?;
    let base = all_roles.iter().find(|r| r.is_default)
        .map(|r| Permissions::from_bits_truncate(r.permissions))
        .unwrap_or_else(Permissions::empty);
    let effective = all_roles.iter()
        .filter(|r| !r.is_default && member.roles.contains(&r.id))
        .map(|r| Permissions::from_bits_truncate(r.permissions))
        .fold(base, |acc, rp| acc | rp);
    if effective.has(Permissions::MANAGE_EMOJIS) || effective.has(Permissions::ADMINISTRATOR) {
        Ok(())
    } else {
        Err(NexusError::MissingPermission { permission: "MANAGE_EMOJIS".into() })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/v1/sticker-packs — list global (server_id IS NULL) packs with their stickers
async fn list_global_packs(
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<Vec<StickerPack>>> {
    let pack_rows = sqlx::query(
        "SELECT id::text AS id, name, description, is_premium \
         FROM sticker_packs WHERE server_id IS NULL ORDER BY name",
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    let mut result = Vec::new();
    for row in &pack_rows {
        let pack_id_str: String = row.try_get("id").unwrap_or_default();
        let pack_id: Uuid = pack_id_str.parse().unwrap_or_default();
        let stickers = fetch_pack_stickers(&state, pack_id).await?;
        result.push(StickerPack {
            id: pack_id,
            name: row.try_get("name").unwrap_or_default(),
            description: row.try_get::<Option<String>, _>("description").unwrap_or(None),
            server_id: None,
            is_premium: row.try_get::<bool, _>("is_premium").unwrap_or(false),
            stickers,
        });
    }
    Ok(Json(result))
}

/// GET /api/v1/servers/:server_id/stickers
async fn list_server_stickers(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
) -> NexusResult<Json<Vec<Sticker>>> {
    let _ = members::find_member(&state.db.pool, auth.user_id, server_id)
        .await?
        .ok_or(NexusError::Forbidden)?;

    let rows = sqlx::query(
        r#"SELECT id::text AS id, pack_id::text AS pack_id, name, description,
                  asset_url, "type" AS sticker_type, created_at::text AS created_at
           FROM stickers WHERE server_id = $1::uuid ORDER BY name"#,
    )
    .bind(server_id.to_string())
    .fetch_all(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    let stickers = rows.iter().filter_map(|r| {
        let id: Uuid = r.try_get::<String, _>("id").unwrap_or_default().parse().ok()?;
        let pack_id: Uuid = r.try_get::<String, _>("pack_id").unwrap_or_default().parse().ok()?;
        let created_at = r.try_get::<String, _>("created_at").ok()
            .as_deref().and_then(parse_pg_dt).unwrap_or_else(Utc::now);
        Some(Sticker {
            id,
            pack_id,
            name: r.try_get("name").unwrap_or_default(),
            description: r.try_get::<Option<String>, _>("description").unwrap_or(None),
            asset_url: r.try_get("asset_url").unwrap_or_default(),
            sticker_type: r.try_get("sticker_type").unwrap_or_else(|_| "png".into()),
            created_at,
        })
    }).collect();

    Ok(Json(stickers))
}

/// POST /api/v1/servers/:server_id/stickers — multipart upload
async fn upload_sticker(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    mut multipart: Multipart,
) -> NexusResult<Json<Sticker>> {
    require_manage_emojis(&state, auth.user_id, server_id).await?;

    // Count existing server stickers — max 60
    let count_row = sqlx::query(
        "SELECT COUNT(*) AS cnt FROM stickers WHERE server_id = $1::uuid",
    )
    .bind(server_id.to_string())
    .fetch_one(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    let count: i64 = count_row.try_get::<i64, _>("cnt").unwrap_or(0);
    if count >= 60 {
        return Err(NexusError::Validation {
            message: "Server has reached the 60-sticker limit".into(),
        });
    }

    let mut name = String::new();
    let mut description: Option<String> = None;
    let mut sticker_type = "png".to_string();
    let mut asset_bytes: Vec<u8> = Vec::new();
    let mut filename = "sticker.png".to_string();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        NexusError::Validation { message: format!("multipart error: {e}") }
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "name" => {
                name = field.text().await.map_err(|e| NexusError::Validation { message: e.to_string() })?;
            }
            "description" => {
                let text = field.text().await.map_err(|e| NexusError::Validation { message: e.to_string() })?;
                description = Some(text);
            }
            "type" => {
                sticker_type = field.text().await.map_err(|e| NexusError::Validation { message: e.to_string() })?;
            }
            "file" => {
                if let Some(fname) = field.file_name() {
                    filename = fname.to_string();
                }
                asset_bytes = field.bytes().await.map_err(|e| NexusError::Validation { message: e.to_string() })?.to_vec();
            }
            _ => {}
        }
    }

    if name.trim().is_empty() || name.len() > 64 {
        return Err(NexusError::Validation { message: "name must be 1–64 characters".into() });
    }
    if asset_bytes.is_empty() {
        return Err(NexusError::Validation { message: "file is required".into() });
    }
    if !["png", "apng", "lottie"].contains(&sticker_type.as_str()) {
        return Err(NexusError::Validation { message: "type must be png, apng, or lottie".into() });
    }

    // Upload to storage
    let sticker_id = snowflake::generate_id();
    let ext = filename.rsplit('.').next().unwrap_or("png").to_lowercase();
    let storage_key = format!("stickers/{}/{}.{}", server_id, sticker_id, ext);
    let media_type = match sticker_type.as_str() {
        "apng" => "image/apng",
        "lottie" => "application/json",
        _ => "image/png",
    };
    state
        .storage
        .put_object(&storage_key, asset_bytes, media_type)
        .await
        .map_err(|e| NexusError::Internal(e))?;
    // 10-year presigned URL effectively serves as a permanent CDN URL for stickers
    let asset_url = state
        .storage
        .presigned_get_url(&storage_key, 3600 * 24 * 365 * 10)
        .await
        .unwrap_or_else(|_| storage_key.clone());

    // Ensure a server-owned pack exists
    let pack_id = ensure_server_pack(&state, server_id).await?;

    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO stickers (id, pack_id, server_id, name, description, asset_url, "type", creator_id, created_at)
           VALUES ($1::uuid, $2::uuid, $3::uuid, $4, $5, $6, $7, $8::uuid, $9::timestamptz)"#,
    )
    .bind(sticker_id.to_string())
    .bind(pack_id.to_string())
    .bind(server_id.to_string())
    .bind(name.trim().to_string())
    .bind(description.clone())
    .bind(&asset_url)
    .bind(&sticker_type)
    .bind(auth.user_id.to_string())
    .bind(now.to_rfc3339())
    .execute(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    let sticker = Sticker {
        id: sticker_id,
        pack_id,
        name: name.trim().to_string(),
        description,
        asset_url,
        sticker_type,
        created_at: now,
    };

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::GUILD_STICKERS_UPDATE.into(),
        data: serde_json::json!({ "guild_id": server_id, "sticker": &sticker }),
        server_id: Some(server_id),
        channel_id: None,
        user_id: Some(auth.user_id),
    });

    Ok(Json(sticker))
}

/// PATCH /api/v1/servers/:server_id/stickers/:sticker_id
async fn rename_sticker(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((server_id, sticker_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<RenameRequest>,
) -> NexusResult<Json<serde_json::Value>> {
    require_manage_emojis(&state, auth.user_id, server_id).await?;
    if body.name.trim().is_empty() || body.name.len() > 64 {
        return Err(NexusError::Validation { message: "name must be 1–64 characters".into() });
    }
    sqlx::query(
        "UPDATE stickers SET name = $1 WHERE id = $2::uuid AND server_id = $3::uuid",
    )
    .bind(body.name.trim().to_string())
    .bind(sticker_id.to_string())
    .bind(server_id.to_string())
    .execute(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/v1/servers/:server_id/stickers/:sticker_id
async fn delete_sticker(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((server_id, sticker_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    require_manage_emojis(&state, auth.user_id, server_id).await?;

    sqlx::query(
        "DELETE FROM stickers WHERE id = $1::uuid AND server_id = $2::uuid",
    )
    .bind(sticker_id.to_string())
    .bind(server_id.to_string())
    .execute(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    let _ = state.gateway_tx.send(GatewayEvent {
        event_type: event_types::GUILD_STICKERS_UPDATE.into(),
        data: serde_json::json!({ "guild_id": server_id, "deleted_sticker_id": sticker_id }),
        server_id: Some(server_id),
        channel_id: None,
        user_id: Some(auth.user_id),
    });

    Ok(Json(serde_json::json!({ "ok": true })))
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

async fn fetch_pack_stickers(state: &AppState, pack_id: Uuid) -> NexusResult<Vec<Sticker>> {
    let rows = sqlx::query(
        r#"SELECT id::text AS id, pack_id::text AS pack_id, name, description,
                  asset_url, "type" AS sticker_type, created_at::text AS created_at
           FROM stickers WHERE pack_id = $1::uuid ORDER BY name"#,
    )
    .bind(pack_id.to_string())
    .fetch_all(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    let stickers = rows.iter().filter_map(|r| {
        let id: Uuid = r.try_get::<String, _>("id").unwrap_or_default().parse().ok()?;
        let pid: Uuid = r.try_get::<String, _>("pack_id").unwrap_or_default().parse().ok()?;
        let created_at = r.try_get::<String, _>("created_at").ok()
            .as_deref().and_then(parse_pg_dt).unwrap_or_else(Utc::now);
        Some(Sticker {
            id,
            pack_id: pid,
            name: r.try_get("name").unwrap_or_default(),
            description: r.try_get::<Option<String>, _>("description").unwrap_or(None),
            asset_url: r.try_get("asset_url").unwrap_or_default(),
            sticker_type: r.try_get("sticker_type").unwrap_or_else(|_| "png".into()),
            created_at,
        })
    }).collect();

    Ok(stickers)
}

/// Find or create the default server sticker pack for `server_id`.
async fn ensure_server_pack(state: &AppState, server_id: Uuid) -> NexusResult<Uuid> {
    let existing = sqlx::query(
        "SELECT id::text AS id FROM sticker_packs WHERE server_id = $1::uuid LIMIT 1",
    )
    .bind(server_id.to_string())
    .fetch_optional(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;

    if let Some(row) = existing {
        let id_str: String = row.try_get("id").unwrap_or_default();
        if let Ok(id) = id_str.parse::<Uuid>() {
            return Ok(id);
        }
    }

    let pack_id = snowflake::generate_id();
    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Server".into() })?;
    sqlx::query(
        "INSERT INTO sticker_packs (id, name, server_id, is_premium) VALUES ($1::uuid, $2, $3::uuid, FALSE)",
    )
    .bind(pack_id.to_string())
    .bind(format!("{} Stickers", server.name))
    .bind(server_id.to_string())
    .execute(&state.db.pool)
    .await
    .map_err(NexusError::Database)?;
    Ok(pack_id)
}
