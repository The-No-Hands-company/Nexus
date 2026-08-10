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
use nexus_common::models::phantom::PhantomKeyPair;
use nexus_db::repository::phantom as phantom_repo;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{AppState, middleware::AuthContext};

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
    Extension(auth): Extension<AuthContext>,
) -> Result<(StatusCode, Json<PhantomIdentityResponse>), StatusCode> {
    let user_id = auth.user_id;
    let pool = &state.db.pool;

    // Already has one? Generating again would orphan the existing key pair and
    // silently invalidate every signature made with it, so return what exists.
    let existing = phantom_repo::get_identity(pool, user_id)
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

    let username = sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = $1::uuid")
        .bind(user_id.to_string())
        .fetch_one(pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let keys = PhantomKeyPair::generate(user_id, &username);

    // One statement writes the identity and its secrets together. This used to
    // be an INSERT that omitted the secrets followed by a separate UPDATE that
    // added them: a crash between the two left an identity whose keys could
    // never be recovered, and whose DID was already published.
    let stored = phantom_repo::insert_identity(pool, &keys.identity, &keys.secrets)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // ON CONFLICT DO NOTHING yields no row when another request created the
    // identity first; that request's keys are authoritative, so read them back
    // rather than reporting keys we did not store.
    let identity = match stored {
        Some(identity) => identity,
        None => phantom_repo::get_identity(pool, user_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
    };

    Ok((
        StatusCode::CREATED,
        Json(PhantomIdentityResponse {
            did: identity.did,
            kem_public: identity.kem_public,
            signing_public: identity.signing_public,
            created_at: identity.created_at.to_rfc3339(),
        }),
    ))
}

/// Get a user's public Phantom identity.
async fn get_user_phantom(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<PhantomPublicResponse>, StatusCode> {
    let identity = phantom_repo::get_identity(&state.db.pool, user_id)
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
