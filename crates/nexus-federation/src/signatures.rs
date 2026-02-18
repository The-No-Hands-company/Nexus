//! Federation request signing and verification.
//!
//! All server-to-server HTTP requests must carry a signed Authorization header:
//!
//! ```text
//! Authorization: NexusFederation origin="nexus.example.com",
//!                key="ed25519:3f9a2c",
//!                sig="<base64url-encoded-signature>"
//! ```
//!
//! The signed content is the canonical JSON of a request object:
//!
//! ```json
//! {
//!   "method":      "PUT",
//!   "uri":         "/_nexus/federation/v1/send/txnABC",
//!   "origin":      "nexus.example.com",
//!   "destination": "nexus.other.tld",
//!   "content":     { ... }   // only present for PUT/POST
//! }
//! ```
//!
//! The object is serialised as canonical JSON (sorted keys, no extra whitespace)
//! before signing.

use std::collections::BTreeMap;


use serde_json::Value;

use crate::{
    error::FederationError,
    keys::{verify_signature, ServerKeyPair},
};

// Maximum allowed clock skew between servers (30 seconds).
const MAX_SKEW_SECS: i64 = 30;

// ─── Signing ─────────────────────────────────────────────────────────────────

/// A signed federation request authorization, ready to be serialised into
/// an HTTP `Authorization` header.
#[derive(Debug, Clone)]
pub struct FedAuth {
    pub origin: String,
    pub key_id: String,
    pub sig: String,
}

impl FedAuth {
    /// Build the `Authorization: NexusFederation …` header value.
    pub fn to_header(&self) -> String {
        format!(
            r#"NexusFederation origin="{}",key="{}",sig="{}""#,
            self.origin, self.key_id, self.sig,
        )
    }
}

/// Sign an outbound federation request and return the [`FedAuth`].
///
/// # Arguments
///
/// * `kp`          — this server's signing key pair
/// * `origin`      — this server's name
/// * `destination` — remote server's name
/// * `method`      — HTTP method, uppercase (e.g. `"PUT"`)
/// * `uri`         — request URI path + query (e.g. `"/_nexus/federation/v1/send/txn1"`)
/// * `content`     — request body (pass `None` for GET requests)
pub fn sign_request(
    kp: &ServerKeyPair,
    origin: &str,
    destination: &str,
    method: &str,
    uri: &str,
    content: Option<&Value>,
) -> FedAuth {
    let canonical = build_signing_object(origin, destination, method, uri, content);
    let sig = kp.sign_json(&canonical);
    FedAuth { origin: origin.to_owned(), key_id: kp.key_id.clone(), sig }
}

/// Verify an inbound federation request.
///
/// * `authorization` — raw value of the `Authorization` header
/// * `destination`   — this server's name; must match what the sender put in the signed object
/// * `method`, `uri`, `content` — as received in the HTTP request
/// * `pubkey_base64` — base64url public key fetched from the origin server's key document
pub fn verify_request(
    authorization: &str,
    destination: &str,
    method: &str,
    uri: &str,
    content: Option<&Value>,
    pubkey_base64: &str,
) -> Result<String, FederationError> {
    let parsed = parse_auth_header(authorization)?;
    let canonical =
        build_signing_object(&parsed.origin, destination, method, uri, content);
    verify_signature(pubkey_base64, &parsed.sig, canonical.as_bytes())?;
    Ok(parsed.origin)
}

// ─── Event signing ────────────────────────────────────────────────────────────

/// Sign a federation event JSON object in-place, adding the server signature
/// under `signatures.<server_name>.<key_id>`.
pub fn sign_event(
    kp: &ServerKeyPair,
    server_name: &str,
    event_json: &mut Value,
) -> Result<(), FederationError> {
    // Remove existing signatures and hashes before signing (they aren't part of the payload).
    let mut signing_obj = event_json.clone();
    signing_obj.as_object_mut().map(|obj| {
        obj.remove("signatures");
        obj.remove("hashes");
    });
    let canonical = canonical_json(&signing_obj)?;
    let sig = kp.sign_json(&canonical);

    // Attach signature.
    let sigs = event_json
        .as_object_mut()
        .ok_or_else(|| FederationError::Other(anyhow::anyhow!("event must be a JSON object")))?
        .entry("signatures")
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .unwrap()
        .entry(server_name)
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .unwrap();
    sigs.insert(kp.key_id.clone(), Value::String(sig));
    Ok(())
}

// ─── Internals ───────────────────────────────────────────────────────────────

/// Build the canonical JSON object that is signed for an HTTP request.
fn build_signing_object(
    origin: &str,
    destination: &str,
    method: &str,
    uri: &str,
    content: Option<&Value>,
) -> String {
    let mut map = BTreeMap::new();
    map.insert("method", Value::String(method.to_uppercase()));
    map.insert("uri", Value::String(uri.to_owned()));
    map.insert("origin", Value::String(origin.to_owned()));
    map.insert("destination", Value::String(destination.to_owned()));
    if let Some(body) = content {
        map.insert("content", body.clone());
    }
    // Serialise as canonical JSON (BTreeMap gives sorted keys).
    serde_json::to_string(&map).expect("BTreeMap serialisation is infallible")
}

/// Parse the `NexusFederation` Authorization header.
fn parse_auth_header(header: &str) -> Result<ParsedAuth, FederationError> {
    let header = header
        .strip_prefix("NexusFederation ")
        .ok_or_else(|| FederationError::MalformedAuthHeader("must start with 'NexusFederation '".into()))?;

    let mut origin = None;
    let mut key = None;
    let mut sig = None;

    for part in header.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("origin=\"").and_then(|s| s.strip_suffix('"')) {
            origin = Some(v.to_owned());
        } else if let Some(v) = part.strip_prefix("key=\"").and_then(|s| s.strip_suffix('"')) {
            key = Some(v.to_owned());
        } else if let Some(v) = part.strip_prefix("sig=\"").and_then(|s| s.strip_suffix('"')) {
            sig = Some(v.to_owned());
        }
    }

    Ok(ParsedAuth {
        origin: origin.ok_or_else(|| FederationError::MalformedAuthHeader("missing 'origin'".into()))?,
        key_id: key.ok_or_else(|| FederationError::MalformedAuthHeader("missing 'key'".into()))?,
        sig: sig.ok_or_else(|| FederationError::MalformedAuthHeader("missing 'sig'".into()))?,
    })
}

struct ParsedAuth {
    origin: String,
    key_id: String,
    sig: String,
}

/// Produce canonical JSON (sorted keys, no extra whitespace).
///
/// Nexus canonical JSON is a subset of RFC 7159 following the Matrix canonical
/// JSON spec: keys sorted lexicographically, no trailing spaces/newlines.
pub fn canonical_json(value: &Value) -> Result<String, FederationError> {
    Ok(sort_keys(value).to_string())
}

fn sort_keys(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: serde_json::Map<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), sort_keys(v)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect();
            Value::Object(sorted)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sort_keys).collect()),
        other => other.clone(),
    }
}
