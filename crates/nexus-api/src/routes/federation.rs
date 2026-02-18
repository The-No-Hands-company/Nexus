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
use sqlx::Row as _;
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
    // ── 1. Authenticate ───────────────────────────────────────────────────────
    let origin = match extract_federation_origin(&headers) {
        Ok(o) => o,
        Err(e) => {
            warn!("Rejected federated transaction {}: {}", txn_id, e);
            return (StatusCode::UNAUTHORIZED, Json(json!({ "error": e }))).into_response();
        }
    };

    debug!("Received federation transaction {} from {}", txn_id, origin);

    // ── 2. Idempotency: skip already-processed transactions ───────────────────
    match sqlx::query(
        "SELECT 1 FROM federation_txn_log \
         WHERE txn_id = $1 AND origin_server = $2 \
         LIMIT 1",
    )
    .bind(&txn_id)
    .bind(&origin)
    .fetch_optional(&state.db.pg)
    .await
    {
        Ok(Some(_)) => {
            debug!("Txn {} from {} already processed — replying idempotently", txn_id, origin);
            return (StatusCode::OK, Json(json!({}))).into_response();
        }
        Ok(None) => {}
        Err(e) => warn!("Failed to query txn_log for idempotency: {}", e),
    }

    // ── 3. Upsert origin server in federated_servers ──────────────────────────
    if let Err(e) = sqlx::query(
        "INSERT INTO federated_servers (server_name, last_seen_at) \
         VALUES ($1, NOW()) \
         ON CONFLICT (server_name) DO UPDATE SET last_seen_at = NOW()",
    )
    .bind(&origin)
    .execute(&state.db.pg)
    .await
    {
        warn!("Failed to upsert federated server {}: {}", origin, e);
    }

    // ── 4. Load verify keys for the origin server ─────────────────────────────
    let verify_keys = load_server_verify_keys(&state.db.pg, &origin).await;

    // ── 5. Process each PDU ───────────────────────────────────────────────────
    let pdus = body
        .get("pdus")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let edu_count = body
        .get("edus")
        .and_then(Value::as_array)
        .map(|a| a.len() as i32)
        .unwrap_or(0);
    let pdu_count = pdus.len() as i32;
    let mut accepted = 0i32;

    for pdu in &pdus {
        match process_pdu(&state.db.pg, &origin, &txn_id, &verify_keys, pdu).await {
            Ok(true) => accepted += 1,
            Ok(false) => debug!("PDU from {} was a duplicate (already stored)", origin),
            Err(e) => warn!("Rejected PDU from {}: {}", origin, e),
        }
    }

    info!(
        "Federation txn {} from {}: {}/{} PDUs accepted, {} EDUs",
        txn_id, origin, accepted, pdu_count, edu_count
    );

    // ── 6. Log the transaction (idempotent guard for future retries) ──────────
    if let Err(e) = sqlx::query(
        "INSERT INTO federation_txn_log \
         (txn_id, origin_server, pdu_count, edu_count) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (txn_id, origin_server) DO NOTHING",
    )
    .bind(&txn_id)
    .bind(&origin)
    .bind(pdu_count)
    .bind(edu_count)
    .execute(&state.db.pg)
    .await
    {
        warn!("Failed to write federation txn log: {}", e);
    }

    (StatusCode::OK, Json(json!({}))).into_response()
}

// ─── PDU helpers ─────────────────────────────────────────────────────────────

/// Process a single incoming PDU:
///
/// 1. Verify the Ed25519 signature if verify keys are available.
/// 2. Persist to `federated_events` (idempotent: ON CONFLICT event_id DO NOTHING).
///
/// Returns `Ok(true)` if newly persisted, `Ok(false)` if duplicate, `Err` if rejected.
async fn process_pdu(
    pool: &sqlx::PgPool,
    origin: &str,
    txn_id: &str,
    verify_keys: &serde_json::Map<String, Value>,
    pdu: &Value,
) -> Result<bool, anyhow::Error> {
    let event_id = pdu
        .get("event_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("PDU missing event_id"))?;
    let room_id = pdu
        .get("room_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("PDU missing room_id"))?;
    let event_type = pdu.get("type").and_then(Value::as_str).unwrap_or("nexus.unknown");
    let sender = pdu.get("sender").and_then(Value::as_str).unwrap_or(origin);
    let origin_server_ts = pdu.get("origin_server_ts").and_then(Value::as_i64).unwrap_or(0);
    let content = pdu
        .get("content")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));
    let signatures = pdu
        .get("signatures")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));

    // Verify signature when we have the origin's public key(s).
    if !verify_keys.is_empty() {
        verify_pdu_signature(pdu, origin, verify_keys)?;
    } else {
        debug!(
            "No cached verify keys for {} — persisting PDU {} without sig check",
            origin, event_id
        );
    }

    // Persist (ON CONFLICT handles duplicate event IDs gracefully).
    let result = sqlx::query(
        "INSERT INTO federated_events \
         (event_id, room_id, event_type, sender, origin_server, \
          origin_server_ts, content, signatures, txn_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         ON CONFLICT (event_id) DO NOTHING",
    )
    .bind(event_id)
    .bind(room_id)
    .bind(event_type)
    .bind(sender)
    .bind(origin)
    .bind(origin_server_ts)
    .bind(&content)
    .bind(&signatures)
    .bind(txn_id)
    .execute(pool)
    .await?;

    // rows_affected == 0 means the event was already stored (conflict).
    Ok(result.rows_affected() > 0)
}

/// Verify the Ed25519 signature on a PDU against the origin server's verify keys.
fn verify_pdu_signature(
    pdu: &Value,
    origin: &str,
    verify_keys: &serde_json::Map<String, Value>,
) -> Result<(), anyhow::Error> {
    let sig_for_origin = pdu
        .get("signatures")
        .and_then(|s| s.get(origin))
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("PDU has no signature for origin {}", origin))?;

    // Pick the first ed25519 key/sig pair.
    let (key_id, sig_b64) = sig_for_origin
        .iter()
        .find(|(k, _)| k.starts_with("ed25519:"))
        .map(|(k, v)| (k.as_str(), v.as_str().unwrap_or("")))
        .ok_or_else(|| anyhow::anyhow!("No ed25519 signature in PDU from {}", origin))?;

    let pubkey_b64 = verify_keys
        .get(key_id)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Unknown key {} for {}", key_id, origin))?;

    // Canonical JSON of PDU without `signatures` / `unsigned` fields.
    let mut pdu_for_signing = pdu.clone();
    if let Value::Object(ref mut m) = pdu_for_signing {
        m.remove("signatures");
        m.remove("unsigned");
    }
    let canonical = nexus_federation::signatures::canonical_json(&pdu_for_signing)
        .map_err(|e| anyhow::anyhow!("canonical_json error: {}", e))?;

    nexus_federation::keys::verify_signature(pubkey_b64, sig_b64, canonical.as_bytes())
        .map_err(|_| anyhow::anyhow!("Signature check failed for {} (key {})", origin, key_id))?;

    Ok(())
}

/// Load the cached verify keys (`key_id → base64_pubkey`) for a remote server
/// from the `federated_servers` table.
async fn load_server_verify_keys(
    pool: &sqlx::PgPool,
    server_name: &str,
) -> serde_json::Map<String, Value> {
    let row = sqlx::query(
        "SELECT verify_keys FROM federated_servers WHERE server_name = $1",
    )
    .bind(server_name)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some(row) = row {
        let val: Value = row.try_get("verify_keys").unwrap_or_default();
        if let Value::Object(m) = val {
            return m;
        }
    }
    Default::default()
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
