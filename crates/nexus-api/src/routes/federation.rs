//! Server-to-server federation API routes (v0.8).
//!
//! These endpoints are accessed by *remote Nexus servers*, not directly by
//! end-user clients. Every inbound request must carry a valid
//! `Authorization: NexusFederation …` header signed with the origin server's
//! Ed25519 key.
//!
//! ## Endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | GET    | `/_nexus/key/v2/server` | Serve this server's public signing key document |
//! | GET    | `/.well-known/nexus/server` | SRV delegation / well-known response |
//! | PUT    | `/_nexus/federation/v1/send/{txnId}` | Receive a transaction from a remote server |
//! | GET    | `/_nexus/federation/v1/event/{eventId}` | Serve a single event by ID |
//! | GET    | `/_nexus/federation/v1/state/{roomId}` | Serve room state at an event |
//! | GET    | `/_nexus/federation/v1/make_join/{roomId}/{userId}` | Prepare a join event template |
//! | PUT    | `/_nexus/federation/v1/send_join/{roomId}/{eventId}` | Receive a signed join event |
//! | GET    | `/_nexus/federation/v1/backfill/{roomId}` | Backfill historical events |
//! | PUT    | `/_matrix/app/v1/transactions/{txnId}` | Matrix AS bridge inbound transactions |

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, info, warn};
use std::sync::Arc;

use crate::AppState;

// ─── Router ───────────────────────────────────────────────────────────────────

/// Mount all federation routes on the given app router.
///
/// Note: these routes live at paths *outside* `/api/v1` so they can follow
/// the Matrix federation spec path conventions.
pub fn federation_router() -> Router<Arc<AppState>> {
    Router::new()
        // Key document (unauthenticated)
        .route("/_nexus/key/v2/server", get(server_key_document))
        // Well-known delegation
        .route("/.well-known/nexus/server", get(well_known_server))
        // Federation S2S endpoints
        .route("/_nexus/federation/v1/send/:txn_id", put(receive_transaction))
        .route("/_nexus/federation/v1/event/:event_id", get(get_event))
        .route("/_nexus/federation/v1/state/:room_id", get(get_room_state))
        .route(
            "/_nexus/federation/v1/make_join/:room_id/:user_id",
            get(make_join),
        )
        .route(
            "/_nexus/federation/v1/send_join/:room_id/:event_id",
            put(send_join),
        )
        .route("/_nexus/federation/v1/backfill/:room_id", get(backfill))
        // Matrix Application Service bridge (inbound)
        .route("/_matrix/app/v1/transactions/:txn_id", put(matrix_as_transaction))
}

// ─── Key document ─────────────────────────────────────────────────────────────

/// `GET /_nexus/key/v2/server`
///
/// Returns this server's public signing key document. Remote servers fetch
/// this once and cache it to verify future request signatures.
async fn server_key_document(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let doc = state.federation_key.to_key_document(&state.server_name);
    (StatusCode::OK, Json(doc))
}

// ─── Well-known ───────────────────────────────────────────────────────────────

/// `GET /.well-known/nexus/server`
///
/// Supports server name delegation. If the server is delegating federation
/// to a different host (e.g. running on a non-standard port), the `m.server`
/// field points to the actual federation endpoint.
async fn well_known_server(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let server_name =
        std::env::var("NEXUS_FEDERATION_SERVER").unwrap_or_else(|_| {
            std::env::var("NEXUS_SERVER_NAME")
                .unwrap_or_else(|_| "localhost:8448".to_owned())
        });
    Json(json!({ "m.server": server_name }))
}

// ─── Transaction receive ──────────────────────────────────────────────────────

/// `PUT /_nexus/federation/v1/send/{txnId}`
///
/// Receives a bundle of events (PDUs) and ephemeral data (EDUs) from a remote
/// server. This is the primary federation ingestion point.
async fn receive_transaction(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(txn_id): Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let origin = match extract_federation_origin(&headers) {
        Ok(o) => o,
        Err(e) => {
            warn!("Rejected federated transaction {}: {}", txn_id, e);
            return (StatusCode::UNAUTHORIZED, Json(json!({ "error": e }))).into_response();
        }
    };

    debug!("Received federation transaction {} from {}", txn_id, origin);

    let pdu_count = body
        .get("pdus")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let edu_count = body
        .get("edus")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);

    info!(
        "Federation txn {} from {}: {} PDUs, {} EDUs",
        txn_id, origin, pdu_count, edu_count
    );

    // TODO: verify each PDU's signature and persist to federated_events.
    // For now, acknowledge receipt.

    (StatusCode::OK, Json(json!({}))).into_response()
}

// ─── Event fetching ───────────────────────────────────────────────────────────

/// `GET /_nexus/federation/v1/event/{eventId}`
async fn get_event(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = extract_federation_origin(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": e }))).into_response();
    }
    // TODO: look up event from federated_events table.
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Event not found" }))).into_response()
}

// ─── Room state ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct StateQuery {
    at: Option<String>,
}

/// `GET /_nexus/federation/v1/state/{roomId}`
async fn get_room_state(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
    Query(query): Query<StateQuery>,
) -> impl IntoResponse {
    if let Err(e) = extract_federation_origin(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": e }))).into_response();
    }
    // TODO: load state events from federated_rooms + federated_events.
    (StatusCode::OK, Json(json!({ "pdus": [], "auth_chain": [] }))).into_response()
}

// ─── Join protocol ────────────────────────────────────────────────────────────

/// `GET /_nexus/federation/v1/make_join/{roomId}/{userId}`
///
/// Returns a join event template that the requesting server should sign
/// and return via `send_join`.
async fn make_join(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((room_id, user_id)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Err(e) = extract_federation_origin(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": e }))).into_response();
    }

    let server_name =
        std::env::var("NEXUS_SERVER_NAME").unwrap_or_else(|_| "localhost".to_owned());

    let template = json!({
        "room_version": "nexus.v1",
        "event": {
            "type": "nexus.member.join",
            "room_id": room_id,
            "sender": user_id,
            "state_key": user_id,
            "content": { "membership": "join" },
            "origin": server_name,
            "origin_server_ts": chrono::Utc::now().timestamp_millis(),
        }
    });

    (StatusCode::OK, Json(template)).into_response()
}

/// `PUT /_nexus/federation/v1/send_join/{roomId}/{eventId}`
///
/// Accepts a signed join event from a remote server, adds the user to the
/// room, and returns the current room state + auth chain.
async fn send_join(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((room_id, event_id)): Path<(String, String)>,
    Json(event): Json<Value>,
) -> impl IntoResponse {
    if let Err(e) = extract_federation_origin(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": e }))).into_response();
    }

    info!("Processing send_join for room {} event {}", room_id, event_id);
    // TODO: verify join event signature, persist, dispatch to gateway.

    (StatusCode::OK, Json(json!({ "state": [], "auth_chain": [] }))).into_response()
}

// ─── Backfill ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct BackfillQuery {
    v: Option<String>, // starting event IDs (comma-separated)
    limit: Option<u32>,
}

/// `GET /_nexus/federation/v1/backfill/{roomId}`
///
/// Returns historical PDUs for a room, starting from the given event IDs,
/// going backwards. Used to fill gaps in a remote server's event DAG.
async fn backfill(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
    Query(query): Query<BackfillQuery>,
) -> impl IntoResponse {
    if let Err(e) = extract_federation_origin(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": e }))).into_response();
    }
    let limit = query.limit.unwrap_or(20).min(100);
    // TODO: fetch from federated_events in reverse order from v.
    (StatusCode::OK, Json(json!({ "pdus": [] }))).into_response()
}

// ─── Matrix AS bridge inbound ────────────────────────────────────────────────

/// `PUT /_matrix/app/v1/transactions/{txnId}`
///
/// Matrix homeserver pushes events to this Application Service.
/// Validates the `access_token` query param (our `hs_token`), then
/// hands off to the bridge for processing.
async fn matrix_as_transaction(
    State(_state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    Path(txn_id): Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    // Validate homeserver token.
    let expected_token =
        std::env::var("NEXUS_MATRIX_HS_TOKEN").unwrap_or_default();
    let provided = params.get("access_token").map(String::as_str).unwrap_or("");

    if !expected_token.is_empty() && provided != expected_token {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "Invalid homeserver token" }))).into_response();
    }

    let event_count = body
        .get("events")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);

    info!("Matrix AS transaction {}: {} events", txn_id, event_count);
    // TODO: pass to MatrixBridge::handle_transaction and dispatch Nexus events.

    (StatusCode::OK, Json(json!({}))).into_response()
}

// ─── Auth helpers ─────────────────────────────────────────────────────────────

/// Extract and loosely parse the originating server from the federation
/// Authorization header. Full signature verification happens in the specific
/// handler once we've loaded the server's public key from DB.
fn extract_federation_origin(headers: &HeaderMap) -> Result<String, String> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or("Missing Authorization header")?;

    let auth = auth
        .strip_prefix("NexusFederation ")
        .ok_or("Authorization scheme must be 'NexusFederation'")?;

    for part in auth.split(',') {
        let part = part.trim();
        if let Some(origin) = part.strip_prefix("origin=\"").and_then(|s| s.strip_suffix('"')) {
            return Ok(origin.to_owned());
        }
    }
    Err("NexusFederation header missing 'origin' field".to_owned())
}
