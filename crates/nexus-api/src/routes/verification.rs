//! Device verification routes — safety numbers & QR verification.
//!
//! GET    /users/:user_id/devices/:device_id/safety-number — Compute safety number
//! POST   /users/:user_id/devices/:device_id/verify        — Record a verification
//! GET    /users/@me/verifications                          — List my verifications
//! DELETE /users/:user_id/devices/:device_id/verify        — Remove verification record

use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use nexus_common::{
    crypto::compute_safety_number,
    error::{NexusError, NexusResult},
    models::crypto::{DeviceVerification, SafetyNumberResponse, VerifyDeviceRequest},
};
use nexus_db::repository::keystore;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/users/{user_id}/devices/{device_id}/safety-number",
            get(get_safety_number),
        )
        .route(
            "/users/{user_id}/devices/{device_id}/verify",
            post(verify_device).delete(remove_verification),
        )
        .route("/users/@me/verifications", get(list_my_verifications))
        .route_layer(middleware::from_fn(crate::middleware::auth_middleware))
}

// ============================================================
// GET /users/:user_id/devices/:device_id/safety-number
// ============================================================

/// Returns the safety number between the authenticated user's identity and
/// the target device's identity. Clients compare this number out-of-band
/// (in person, via phone, etc.) to verify there is no MITM.
async fn get_safety_number(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((target_user_id, target_device_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<SafetyNumberResponse>> {
    // Fetch the target device
    let target_device = keystore::find_device(&state.db.pg, target_device_id)
        .await
        .map_err(|e| NexusError::Internal(e))?
        .ok_or(NexusError::NotFound {
            resource: "Device".into(),
        })?;

    if target_device.user_id != target_user_id {
        return Err(NexusError::NotFound {
            resource: "Device".into(),
        });
    }

    // Fetch the caller's first/primary device for their identity key
    let my_devices = keystore::list_devices(&state.db.pg, auth.user_id)
        .await
        .map_err(|e| NexusError::Internal(e))?;

    let my_device = my_devices.into_iter().next().ok_or(NexusError::Validation {
        message: "Register a device before computing safety numbers".into(),
    })?;

    let fingerprint = compute_safety_number(
        auth.user_id,
        &my_device.identity_key,
        target_user_id,
        &target_device.identity_key,
    )
    .map_err(|e| NexusError::Validation {
        message: format!("Failed to compute safety number: {e}"),
    })?;

    Ok(Json(SafetyNumberResponse {
        local_identity_key: my_device.identity_key,
        remote_identity_key: target_device.identity_key,
        fingerprint,
    }))
}

// ============================================================
// POST /users/:user_id/devices/:device_id/verify
// ============================================================

async fn verify_device(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((_target_user_id, device_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<VerifyDeviceRequest>,
) -> NexusResult<Json<DeviceVerification>> {
    // Ensure device exists
    keystore::find_device(&state.db.pg, device_id)
        .await
        .map_err(|e| NexusError::Internal(e))?
        .ok_or(NexusError::NotFound {
            resource: "Device".into(),
        })?;

    let method_str = match body.method {
        nexus_common::models::crypto::VerificationMethod::SafetyNumber => "safety_number",
        nexus_common::models::crypto::VerificationMethod::QrScan => "qr_scan",
        nexus_common::models::crypto::VerificationMethod::Emoji => "emoji",
    };

    let verification = keystore::verify_device(&state.db.pg, auth.user_id, device_id, method_str)
        .await
        .map_err(|e| NexusError::Internal(e))?;

    Ok(Json(verification))
}

// ============================================================
// DELETE /users/:user_id/devices/:device_id/verify
// ============================================================

async fn remove_verification(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((_target_user_id, device_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<()> {
    sqlx::query(
        "DELETE FROM device_verifications WHERE verifier_id = $1 AND target_device_id = $2",
    )
    .bind(auth.user_id)
    .bind(device_id)
    .execute(&state.db.pg)
    .await
    .map_err(|e| NexusError::Database(e))?;

    Ok(())
}

// ============================================================
// GET /users/@me/verifications
// ============================================================

async fn list_my_verifications(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> NexusResult<Json<Vec<DeviceVerification>>> {
    let verifications = keystore::list_verifications(&state.db.pg, auth.user_id)
        .await
        .map_err(|e| NexusError::Internal(e))?;
    Ok(Json(verifications))
}
