//! Device & key management routes — register devices, upload key material.
//!
//! POST   /devices                           — Register a new device + upload initial keys
//! GET    /devices                           — List my devices
//! GET    /devices/:device_id                — Get device info
//! DELETE /devices/:device_id               — Revoke a device
//! POST   /devices/:device_id/signed-pre-key — Rotate signed pre-key
//! POST   /devices/:device_id/one-time-pre-keys — Upload more OTPks
//! GET    /devices/:device_id/one-time-pre-keys/count — Remaining OTPk count
//! GET    /users/:user_id/key-bundle         — Fetch key bundles for all devices (X3DH initiator)
//! GET    /users/:user_id/devices/:device_id/key-bundle — Fetch bundle for one device

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::{
    crypto::{validate_identity_key, validate_signature, validate_x25519_key},
    error::{NexusError, NexusResult},
    models::crypto::{
        Device, KeyBundle, OtpkCountResponse, RegisterDeviceRequest, RotateSignedPreKeyRequest,
        UploadOtpkRequest,
    },
};
use nexus_db::repository::keystore;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Own device management
        .route("/devices", post(register_device).get(list_my_devices))
        .route(
            "/devices/:device_id",
            get(get_device).delete(delete_device),
        )
        .route(
            "/devices/:device_id/signed-pre-key",
            post(rotate_signed_pre_key),
        )
        .route(
            "/devices/:device_id/one-time-pre-keys",
            post(upload_one_time_pre_keys),
        )
        .route(
            "/devices/:device_id/one-time-pre-keys/count",
            get(count_one_time_pre_keys),
        )
        // Key bundles (for X3DH initiators)
        .route("/users/:user_id/key-bundle", get(get_all_key_bundles))
        .route(
            "/users/:user_id/devices/:device_id/key-bundle",
            get(get_device_key_bundle),
        )
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

// ============================================================
// POST /devices — Register a new device
// ============================================================

async fn register_device(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterDeviceRequest>,
) -> NexusResult<Json<Device>> {
    // Validate key material before persisting
    validate_identity_key(&body.identity_key).map_err(|e| NexusError::Validation {
        message: format!("identity_key: {e}"),
    })?;
    validate_x25519_key(&body.signed_pre_key, "signed_pre_key").map_err(|e| {
        NexusError::Validation {
            message: format!("signed_pre_key: {e}"),
        }
    })?;
    validate_signature(&body.signed_pre_key_sig).map_err(|e| NexusError::Validation {
        message: format!("signed_pre_key_sig: {e}"),
    })?;

    let device_type_str = body
        .device_type
        .map(|t| serde_json::to_value(t).unwrap_or_default().as_str().unwrap_or("unknown").to_owned())
        .unwrap_or_else(|| "unknown".into());

    let device = keystore::create_device(
        &state.db.pg,
        auth.user_id,
        &body.name,
        &device_type_str,
        &body.identity_key,
        &body.signed_pre_key,
        &body.signed_pre_key_sig,
        body.signed_pre_key_id,
    )
    .await
    .map_err(|e| NexusError::Internal(e))?;

    // Upload initial one-time pre-keys
    if !body.one_time_pre_keys.is_empty() {
        let pairs: Vec<(i32, String)> = body
            .one_time_pre_keys
            .iter()
            .map(|k| (k.key_id, k.public_key.clone()))
            .collect();
        keystore::insert_one_time_pre_keys(&state.db.pg, device.id, &pairs)
            .await
            .map_err(|e| NexusError::Internal(e))?;
    }

    Ok(Json(device))
}

// ============================================================
// GET /devices
// ============================================================

async fn list_my_devices(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<Vec<Device>>> {
    let devices = keystore::list_devices(&state.db.pg, auth.user_id)
        .await
        .map_err(|e| NexusError::Internal(e))?;
    Ok(Json(devices))
}

// ============================================================
// GET /devices/:device_id
// ============================================================

async fn get_device(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> NexusResult<Json<Device>> {
    let device = keystore::find_device(&state.db.pg, device_id)
        .await
        .map_err(|e| NexusError::Internal(e))?
        .ok_or(NexusError::NotFound {
            resource: "Device".into(),
        })?;

    // Allow self-access or any user to see another's public key info
    // (key bundles are public — that's how E2EE works)
    let _ = auth;

    Ok(Json(device))
}

// ============================================================
// DELETE /devices/:device_id
// ============================================================

async fn delete_device(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> NexusResult<()> {
    let device = keystore::find_device(&state.db.pg, device_id)
        .await
        .map_err(|e| NexusError::Internal(e))?
        .ok_or(NexusError::NotFound {
            resource: "Device".into(),
        })?;

    if device.user_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }

    keystore::delete_device(&state.db.pg, device_id)
        .await
        .map_err(|e| NexusError::Internal(e))?;

    Ok(())
}

// ============================================================
// POST /devices/:device_id/signed-pre-key — Rotate signed pre-key
// ============================================================

async fn rotate_signed_pre_key(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Json(body): Json<RotateSignedPreKeyRequest>,
) -> NexusResult<()> {
    let device = keystore::find_device(&state.db.pg, device_id)
        .await
        .map_err(|e| NexusError::Internal(e))?
        .ok_or(NexusError::NotFound {
            resource: "Device".into(),
        })?;

    if device.user_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }

    validate_x25519_key(&body.signed_pre_key, "signed_pre_key").map_err(|e| {
        NexusError::Validation {
            message: format!("signed_pre_key: {e}"),
        }
    })?;
    validate_signature(&body.signed_pre_key_sig).map_err(|e| NexusError::Validation {
        message: format!("signed_pre_key_sig: {e}"),
    })?;

    keystore::rotate_signed_pre_key(
        &state.db.pg,
        device_id,
        &body.signed_pre_key,
        &body.signed_pre_key_sig,
        body.signed_pre_key_id,
    )
    .await
    .map_err(|e| NexusError::Internal(e))?;

    Ok(())
}

// ============================================================
// POST /devices/:device_id/one-time-pre-keys — Upload OTPks
// ============================================================

async fn upload_one_time_pre_keys(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Json(body): Json<UploadOtpkRequest>,
) -> NexusResult<Json<OtpkCountResponse>> {
    let device = keystore::find_device(&state.db.pg, device_id)
        .await
        .map_err(|e| NexusError::Internal(e))?
        .ok_or(NexusError::NotFound {
            resource: "Device".into(),
        })?;

    if device.user_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }

    if body.keys.len() > 1000 {
        return Err(NexusError::Validation {
            message: "Cannot upload more than 1000 one-time pre-keys at once".into(),
        });
    }

    let pairs: Vec<(i32, String)> = body.keys.iter().map(|k| (k.key_id, k.public_key.clone())).collect();
    keystore::insert_one_time_pre_keys(&state.db.pg, device_id, &pairs)
        .await
        .map_err(|e| NexusError::Internal(e))?;

    let remaining = keystore::count_one_time_pre_keys(&state.db.pg, device_id)
        .await
        .map_err(|e| NexusError::Internal(e))?;

    Ok(Json(OtpkCountResponse { device_id, remaining }))
}

// ============================================================
// GET /devices/:device_id/one-time-pre-keys/count
// ============================================================

async fn count_one_time_pre_keys(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> NexusResult<Json<OtpkCountResponse>> {
    let device = keystore::find_device(&state.db.pg, device_id)
        .await
        .map_err(|e| NexusError::Internal(e))?
        .ok_or(NexusError::NotFound {
            resource: "Device".into(),
        })?;

    if device.user_id != auth.user_id {
        return Err(NexusError::Forbidden);
    }

    let remaining = keystore::count_one_time_pre_keys(&state.db.pg, device_id)
        .await
        .map_err(|e| NexusError::Internal(e))?;

    Ok(Json(OtpkCountResponse { device_id, remaining }))
}

// ============================================================
// GET /users/:user_id/key-bundle — All bundles for a user's devices
// ============================================================

async fn get_all_key_bundles(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> NexusResult<Json<Vec<KeyBundle>>> {
    let bundles = keystore::get_all_key_bundles(&state.db.pg, user_id)
        .await
        .map_err(|e| NexusError::Internal(e))?;
    Ok(Json(bundles))
}

// ============================================================
// GET /users/:user_id/devices/:device_id/key-bundle
// ============================================================

async fn get_device_key_bundle(
    Extension(_auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((_user_id, device_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<KeyBundle>> {
    let bundle = keystore::get_key_bundle(&state.db.pg, device_id)
        .await
        .map_err(|e| NexusError::Internal(e))?
        .ok_or(NexusError::NotFound {
            resource: "Device".into(),
        })?;
    Ok(Json(bundle))
}
