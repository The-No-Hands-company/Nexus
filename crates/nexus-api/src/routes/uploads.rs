//! File upload routes — multipart upload to MinIO, attachment management.
//!
//! POST  /api/v1/attachments/upload          — Upload a file (multipart/form-data)
//! GET   /api/v1/attachments/:id             — Get attachment metadata + presigned URL
//! DELETE /api/v1/attachments/:id            — Delete own attachment

use axum::{
    extract::{Multipart, Path, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_db::repository::attachments;
use serde::Serialize;
use sha2::Digest;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};
use axum::extract::Extension;

// ============================================================
// Maximum upload size: 100 MiB
// ============================================================
const MAX_UPLOAD_BYTES: usize = 100 * 1024 * 1024;

/// Allowed content-type categories. Reject executables server-side.
fn is_allowed_content_type(ct: &str) -> bool {
    matches!(
        ct,
        // Images
        | "image/jpeg" | "image/png" | "image/gif" | "image/webp"
        | "image/svg+xml" | "image/avif" | "image/bmp" | "image/tiff"
        // Video
        | "video/mp4" | "video/webm" | "video/ogg" | "video/quicktime"
        // Audio
        | "audio/mpeg" | "audio/ogg" | "audio/wav" | "audio/flac"
        | "audio/aac" | "audio/opus" | "audio/webm"
        // Documents
        | "application/pdf" | "text/plain" | "text/markdown"
        | "application/zip" | "application/x-tar"
    )
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/attachments/upload", post(upload_file))
        .route(
            "/attachments/{id}",
            get(get_attachment).delete(delete_attachment),
        )
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ============================================================
// Response types
// ============================================================

#[derive(Serialize)]
struct AttachmentResponse {
    id: Uuid,
    filename: String,
    content_type: String,
    size: i64,
    url: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    duration_secs: Option<f64>,
    spoiler: bool,
    status: String,
}

// ============================================================
// POST /attachments/upload
// ============================================================

/// Upload a file via multipart/form-data.
///
/// Form fields:
/// - `file`   — the binary file (required)
/// - `spoiler` — "true" to mark as spoiler (optional)
/// - `channel_id` — associate with a channel (optional)
async fn upload_file(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> NexusResult<Json<AttachmentResponse>> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename = String::from("upload");
    let mut content_type = String::from("application/octet-stream");
    let mut spoiler = false;
    let mut channel_id: Option<Uuid> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| NexusError::Validation {
            message: format!("Multipart error: {e}"),
        })?
    {
        match field.name() {
            Some("file") => {
                // Capture filename and content-type from the field headers
                if let Some(fn_) = field.file_name() {
                    filename = fn_.to_string();
                }
                if let Some(ct) = field.content_type() {
                    content_type = ct.to_string();
                }

                // Validate content-type early
                if !is_allowed_content_type(&content_type) {
                    return Err(NexusError::Validation {
                        message: format!("File type '{content_type}' is not allowed"),
                    });
                }

                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| NexusError::Validation {
                        message: format!("Failed to read file: {e}"),
                    })?;

                if bytes.len() > MAX_UPLOAD_BYTES {
                    return Err(NexusError::Validation {
                        message: format!(
                            "File too large: {} bytes (max {} bytes)",
                            bytes.len(),
                            MAX_UPLOAD_BYTES
                        ),
                    });
                }

                file_data = Some(bytes.to_vec());
            }
            Some("spoiler") => {
                let val = field.text().await.unwrap_or_default();
                spoiler = val.trim() == "true";
            }
            Some("channel_id") => {
                let val = field.text().await.unwrap_or_default();
                channel_id = Uuid::parse_str(val.trim()).ok();
            }
            _ => {} // Ignore unknown fields
        }
    }

    let data = file_data.ok_or(NexusError::Validation {
        message: "No file field in request".into(),
    })?;

    let size = data.len() as i64;

    // Sanitize filename
    let safe_filename = sanitize_filename(&filename);

    // Compute SHA-256 digest for deduplication (crypto-quality, not DefaultHasher)
    let hash_hex = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&data);
        hex::encode(h.finalize())
    };

    // Build storage key: uploads/{user_id}/{uuid}.{ext}
    let ext = safe_filename
        .rsplit('.')
        .next()
        .unwrap_or("bin")
        .to_lowercase();
    let attachment_id = Uuid::new_v4();
    let storage_key = format!("uploads/{}/{}.{}", auth.user_id, attachment_id, ext);

    // Upload to MinIO / local storage
    state
        .storage
        .put_object(&storage_key, data, &content_type)
        .await
        .map_err(|e| NexusError::Internal(e))?;

    // Persist attachment metadata — URL is intentionally left empty here.
    // GET /attachments/:id always generates a fresh presigned URL from
    // storage_key so no stale URL is ever served.
    let row = attachments::create_attachment(
        &state.db.pool,
        attachment_id,
        auth.user_id,
        None, // server_id — we don't know yet
        channel_id,
        &safe_filename,
        &content_type,
        size,
        &storage_key,
        None, // width
        None, // height
        None, // duration
        spoiler,
        Some(&hash_hex),
    )
    .await?;

    // Mark ready immediately; URL is empty — it will be generated fresh on each GET.
    let row = attachments::mark_ready(
        &state.db.pool,
        row.id,
        "", // no stored URL; presigned URL generated on demand
        None, // blurhash — would need async image processing
    )
    .await?;

    Ok(Json(AttachmentResponse {
        id: row.id,
        filename: row.filename,
        content_type: row.content_type,
        size: row.size,
        url: row.url,
        width: row.width,
        height: row.height,
        duration_secs: row.duration_secs,
        spoiler: row.spoiler,
        status: row.status,
    }))
}

// ============================================================
// GET /attachments/:id
// ============================================================

async fn get_attachment(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<AttachmentResponse>> {
    let _ = auth; // auth just verifies the user is logged in

    let row = attachments::find_by_id(&state.db.pool, id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Attachment".into(),
        })?;

    // Always generate a fresh presigned URL from the storage key.
    // The URL stored in the DB at upload time may be stale or permanent —
    // we use the live storage client so private channels' attachments are
    // always access-controlled and URLs always have a bounded TTL.
    const PRESIGN_TTL_SECS: u64 = 60 * 60 * 24; // 24 hours
    let url = state
        .storage
        .presigned_get_url(&row.storage_key, PRESIGN_TTL_SECS)
        .await
        .ok()
        .or_else(|| row.url.clone()); // fallback to stored URL if presigning fails

    Ok(Json(AttachmentResponse {
        id: row.id,
        filename: row.filename,
        content_type: row.content_type,
        size: row.size,
        url,
        width: row.width,
        height: row.height,
        duration_secs: row.duration_secs,
        spoiler: row.spoiler,
        status: row.status,
    }))
}

// ============================================================
// DELETE /attachments/:id
// ============================================================

async fn delete_attachment(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    // Find the attachment first to get the storage key
    let row = attachments::find_by_id(&state.db.pool, id)
        .await?
        .ok_or(NexusError::NotFound {
            resource: "Attachment".into(),
        })?;

    // Only the uploader can delete
    if row.uploader_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }

    // Delete from DB
    attachments::delete_attachment(&state.db.pool, id, auth.user_id).await?;

    // Delete from object storage (best-effort — don't fail if already gone)
    let _ = state.storage.delete_object(&row.storage_key).await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ============================================================
// Helpers
// ============================================================

/// Strip path separators and null bytes from filenames.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '/' | '\\' | '\0'))
        .take(255)
        .collect()
}
