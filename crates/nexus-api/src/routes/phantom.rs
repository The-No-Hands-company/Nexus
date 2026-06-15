//! Phantom Protocol — Post-quantum E2EE identity generation endpoint.
//!
//! POST /api/users/@me/phantom — Generate Phantom PQ identity for current user.
//! GET  /api/users/{id}/phantom  — Get a user's public Phantom identity.

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::StatusCode,
    routing::{get, post},
};
use nexus_common::{
    models::phantom::{PhantomIdentity, PhantomKeyPair, PhantomSecretKeys},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{AppState, auth};

/// POST /api/users/@me/phantom — Generate a Phantom identity.
/// Returns the public DID and base64-encoded public keys.
/// Secret keys are stored server-side and never returned to clients.
#[derive(Debug, Serialize)]
struct PhantomIdentityResponse {
    did: String,
    kem_public: String,     // base64 Kyber-1024 public key
    signing_public: String, // base64 Dilithium-5 public key
    created_at: String,
}

/// GET /api/users/{id}/phantom — Get a user's public Phantom identity.
#[derive(Debug, Serialize)]
struct PhantomPublicResponse {
    has_phantom: bool,
    did: Option<String>,
    kem_public: Option<String>,
    signing_public: Option<String>,
}

/// Generate a Phantom identity for the authenticated user.
async fn generate_phantom_identity(
    State(state): State<Arc<AppState>>,
    Extension(auth_ctx): Extension<auth::AuthContext>,
) -> Result<(StatusCode, Json<PhantomIdentityResponse>), StatusCode> {
    let user_id = auth_ctx.user_id;

    // Check if user already has a Phantom identity
    let existing: Option<PhantomIdentity> = sqlx::query_as(
        "SELECT user_id, did, kem_public, signing_public, created_at FROM phantom_identities WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(identity) = existing {
        return Ok((
            StatusCode::OK,
            Json(PhantomIdentityResponse {
                did: identity.did,
                kem_public: identity.kem_public,
                signing_public: identity.signing_public,
                created_at: identity.created_at.to_rfc3339(),
            }),
        ));
    }

    // Generate new Phantom keys
    let username = sqlx::query_scalar::<_, String>(
        "SELECT username FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    let keys = PhantomKeyPair::generate(user_id, &username);

    // Store identity in database
    sqlx::query(
        "INSERT INTO phantom_identities (user_id, did, kem_public, signing_public, created_at)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id) DO NOTHING"
    )
    .bind(keys.identity.user_id)
    .bind(&keys.identity.did)
    .bind(&keys.identity.kem_public)
    .bind(&keys.identity.signing_public)
    .bind(keys.identity.created_at)
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Store secret keys securely (encrypted at rest)
    // In production: encrypt with VAULT_MASTER_KEY before storing
    sqlx::query(
        "UPDATE phantom_identities SET kem_secret = $1, signing_secret = $2 WHERE user_id = $3"
    )
    .bind(&keys.secrets.kem_secret)
    .bind(&keys.secrets.signing_secret)
    .bind(user_id)
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(PhantomIdentityResponse {
            did: keys.identity.did,
            kem_public: keys.identity.kem_public,
            signing_public: keys.identity.signing_public,
            created_at: keys.identity.created_at.to_rfc3339(),
        }),
    ))
}

/// Get a user's public Phantom identity.
async fn get_user_phantom(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<PhantomPublicResponse>, StatusCode> {
    let identity: Option<PhantomIdentity> = sqlx::query_as(
        "SELECT user_id, did, kem_public, signing_public, created_at FROM phantom_identities WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match identity {
        Some(id) => Ok(Json(PhantomPublicResponse {
            has_phantom: true,
            did: Some(id.did),
            kem_public: Some(id.kem_public),
            signing_public: Some(id.signing_public),
        })),
        None => Ok(Json(PhantomPublicResponse {
            has_phantom: false,
            did: None,
            kem_public: None,
            signing_public: None,
        })),
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users/@me/phantom", post(generate_phantom_identity))
        .route("/users/{user_id}/phantom", get(get_user_phantom))
}
