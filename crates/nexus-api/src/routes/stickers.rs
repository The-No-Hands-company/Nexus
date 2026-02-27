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
    routing::{delete, get, patch, post},
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
    let packs = sqlx::query!(
        "SELECT id, name, description, is_premium FROM sticker_packs WHERE server_id IS NULL ORDER BY name"
    )
    .fetch_all(&state.db.pool)
    .await?;

    let mut result = Vec::new();
    for p in packs {
        let stickers = fetch_pack_stickers(&state, p.id).await?;
        result.push(StickerPack {
            id: p.id,
            name: p.name,
            description: p.description,
            server_id: None,
            is_premium: p.is_premium,
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

    let stickers = sqlx::query!(
        "SELECT id, pack_id, name, description, asset_url, type AS sticker_type, created_at FROM stickers WHERE server_id = $1 ORDER BY name",
        server_id,
    )
    .fetch_all(&state.db.pool)
    .await?;

    Ok(Json(stickers.into_iter().map(|s| Sticker {
        id: s.id,
        pack_id: s.pack_id,
        name: s.name,
        description: s.description,
        asset_url: s.asset_url,
        sticker_type: s.sticker_type,
        created_at: s.created_at,
    }).collect()))
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
    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM stickers WHERE server_id = $1", server_id
    )
    .fetch_one(&state.db.pool)
    .await?
    .unwrap_or(0);

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
    sqlx::query!(
        "INSERT INTO stickers (id, pack_id, server_id, name, description, asset_url, type, creator_id, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        sticker_id, pack_id, server_id, name.trim(), description.as_deref(), asset_url, sticker_type, auth.user_id, now,
    )
    .execute(&state.db.pool)
    .await?;

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
    sqlx::query!(
        "UPDATE stickers SET name = $1 WHERE id = $2 AND server_id = $3",
        body.name.trim(), sticker_id, server_id,
    )
    .execute(&state.db.pool)
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/v1/servers/:server_id/stickers/:sticker_id
async fn delete_sticker(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((server_id, sticker_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    require_manage_emojis(&state, auth.user_id, server_id).await?;

    sqlx::query!(
        "DELETE FROM stickers WHERE id = $1 AND server_id = $2",
        sticker_id, server_id,
    )
    .execute(&state.db.pool)
    .await?;

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
    let rows = sqlx::query!(
        "SELECT id, pack_id, name, description, asset_url, type AS sticker_type, created_at FROM stickers WHERE pack_id = $1 ORDER BY name",
        pack_id
    )
    .fetch_all(&state.db.pool)
    .await?;
    Ok(rows.into_iter().map(|s| Sticker {
        id: s.id,
        pack_id: s.pack_id,
        name: s.name,
        description: s.description,
        asset_url: s.asset_url,
        sticker_type: s.sticker_type,
        created_at: s.created_at,
    }).collect())
}

/// Find or create the default server sticker pack for `server_id`.
async fn ensure_server_pack(state: &AppState, server_id: Uuid) -> NexusResult<Uuid> {
    if let Some(row) = sqlx::query_scalar!(
        "SELECT id FROM sticker_packs WHERE server_id = $1 LIMIT 1",
        server_id
    )
    .fetch_optional(&state.db.pool)
    .await?
    {
        return Ok(row);
    }

    let pack_id = snowflake::generate_id();
    let server = servers::find_by_id(&state.db.pool, server_id)
        .await?
        .ok_or(NexusError::NotFound { resource: "Server".into() })?;
    sqlx::query!(
        "INSERT INTO sticker_packs (id, name, server_id, is_premium) VALUES ($1, $2, $3, FALSE)",
        pack_id,
        format!("{} Stickers", server.name),
        server_id,
    )
    .execute(&state.db.pool)
    .await?;
    Ok(pack_id)
}
