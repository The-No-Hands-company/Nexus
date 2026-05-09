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
    keys::{ServerKeyPair, verify_signature},
};

// Reserved for future timestamp skew validation in inbound federation auth.
const _MAX_SKEW_SECS: i64 = 30;

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
    #[must_use] 
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
#[must_use] 
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
    FedAuth {
        origin: origin.to_owned(),
        key_id: kp.key_id.clone(),
        sig,
    }
}

/// Verify an inbound federation request.
///
/// * `authorization` — raw value of the `Authorization` header
/// * `destination`   — this server's name; must match what the sender put in the signed object
/// * `method`, `uri`, `content` — as received in the HTTP request
/// * `pubkey_base64` — base64url public key fetched from the origin server's key document
///
/// # Errors
/// Returns an error if the auth header is malformed, signature verification
/// fails, or canonicalisation prerequisites are not met.
pub fn verify_request(
    authorization: &str,
    destination: &str,
    method: &str,
    uri: &str,
    content: Option<&Value>,
    pubkey_base64: &str,
) -> Result<String, FederationError> {
    let parsed = parse_auth_header(authorization)?;
    let canonical = build_signing_object(&parsed.origin, destination, method, uri, content);
    verify_signature(pubkey_base64, &parsed.sig, canonical.as_bytes())?;
    Ok(parsed.origin)
}

// ─── Event signing ────────────────────────────────────────────────────────────

/// Sign a federation event JSON object in-place, adding the server signature
/// under `signatures.<server_name>.<key_id>`.
///
/// # Errors
/// Returns an error if `event_json` is not a JSON object or canonicalisation
/// fails.
///
/// # Panics
/// Panics if the intermediate `signatures` structure is present but not a JSON
/// object as expected by the federation event schema.
pub fn sign_event(
    kp: &ServerKeyPair,
    server_name: &str,
    event_json: &mut Value,
) -> Result<(), FederationError> {
    // Remove existing signatures and hashes before signing (they aren't part of the payload).
    let mut signing_obj = event_json.clone();
    if let Some(obj) = signing_obj.as_object_mut() {
        obj.remove("signatures");
        obj.remove("hashes");
    }
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
    let header = header.strip_prefix("NexusFederation ").ok_or_else(|| {
        FederationError::MalformedAuthHeader("must start with 'NexusFederation '".into())
    })?;

    let mut origin = None;
    let mut key = None;
    let mut sig = None;

    for part in header.split(',') {
        let part = part.trim();
        if let Some(v) = part
            .strip_prefix("origin=\"")
            .and_then(|s| s.strip_suffix('"'))
        {
            origin = Some(v.to_owned());
        } else if let Some(v) = part
            .strip_prefix("key=\"")
            .and_then(|s| s.strip_suffix('"'))
        {
            key = Some(v.to_owned());
        } else if let Some(v) = part
            .strip_prefix("sig=\"")
            .and_then(|s| s.strip_suffix('"'))
        {
            sig = Some(v.to_owned());
        }
    }

    Ok(ParsedAuth {
        origin: origin
            .ok_or_else(|| FederationError::MalformedAuthHeader("missing 'origin'".into()))?,
        _key_id: key.ok_or_else(|| FederationError::MalformedAuthHeader("missing 'key'".into()))?,
        sig: sig.ok_or_else(|| FederationError::MalformedAuthHeader("missing 'sig'".into()))?,
    })
}

#[derive(Debug)]
struct ParsedAuth {
    origin: String,
    _key_id: String,
    sig: String,
}

/// Produce canonical JSON (sorted keys, no extra whitespace).
///
/// Nexus canonical JSON is a subset of RFC 7159 following the Matrix canonical
/// JSON spec: keys sorted lexicographically, no trailing spaces/newlines.
///
/// # Errors
/// Returns an error if canonical JSON generation fails.
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── canonical_json ────────────────────────────────────────────────────────

    #[test]
    fn object_keys_are_sorted() {
        let val = json!({ "z": 1, "a": 2, "m": 3 });
        let result = canonical_json(&val).unwrap();
        // Sorted keys: a, m, z
        let pos_a = result.find("\"a\"").unwrap();
        let pos_m = result.find("\"m\"").unwrap();
        let pos_z = result.find("\"z\"").unwrap();
        assert!(pos_a < pos_m, "a must come before m");
        assert!(pos_m < pos_z, "m must come before z");
    }

    #[test]
    fn nested_object_keys_are_sorted() {
        let val = json!({ "b": { "z": 1, "a": 2 }, "a": 3 });
        let result = canonical_json(&val).unwrap();
        // Outer keys: "a" before "b" (lexicographic)
        let outer_a = result.find("\"a\":3").unwrap();
        let outer_b = result.find("\"b\":").unwrap();
        assert!(outer_a < outer_b, "outer 'a' must sort before 'b'");
        // Inner nested keys: "a" before "z"
        let inner_a = result.find("\"a\":2").unwrap();
        let inner_z = result.find("\"z\":1").unwrap();
        assert!(inner_a < inner_z, "inner 'a' must sort before 'z'");
    }

    #[test]
    fn canonical_json_is_deterministic() {
        let val = json!({ "z": true, "a": [1, 2, 3], "m": null });
        let r1 = canonical_json(&val).unwrap();
        let r2 = canonical_json(&val).unwrap();
        assert_eq!(r1, r2);
    }

    #[test]
    fn same_data_different_key_order_produces_same_canonical() {
        let v1 = json!({ "z": 1, "a": 2 });
        let v2 = json!({ "a": 2, "z": 1 });
        assert_eq!(canonical_json(&v1).unwrap(), canonical_json(&v2).unwrap());
    }

    #[test]
    fn arrays_preserve_element_order() {
        let val = json!({ "items": [3, 1, 2] });
        let result = canonical_json(&val).unwrap();
        // Elements must not be sorted — arrays are order-sensitive
        assert!(
            result.contains("[3,1,2]"),
            "array order must be preserved: {result}"
        );
    }

    #[test]
    fn primitives_round_trip_unchanged() {
        for val in &[
            json!(42),
            json!("hello"),
            json!(true),
            json!(null),
            json!(std::f64::consts::PI),
        ] {
            let result = canonical_json(val).unwrap();
            let reparsed: serde_json::Value = serde_json::from_str(&result).unwrap();
            assert_eq!(
                &reparsed, val,
                "primitive must survive canonical round-trip"
            );
        }
    }

    #[test]
    fn parse_auth_header_valid_nexus_header() {
        // Must start with "NexusFederation " prefix
        let header =
            "NexusFederation origin=\"nexus.example.com\",key=\"ed25519:key1\",sig=\"abc123\"";
        let result = parse_auth_header(header);
        assert!(result.is_ok(), "valid header should parse: {result:?}");
        let parsed = result.unwrap();
        assert_eq!(parsed.origin, "nexus.example.com");
        assert_eq!(parsed.sig, "abc123");
    }

    #[test]
    fn parse_auth_header_missing_required_fields_errors() {
        // Missing sig field
        let bad = "NexusFederation origin=\"nexus.example.com\",key=\"ed25519:key1\"";
        assert!(parse_auth_header(bad).is_err());
    }

    #[test]
    fn parse_auth_header_wrong_prefix_errors() {
        // Must start with "NexusFederation ", not "X-Nexus-Signature"
        let bad = "X-Nexus-Signature origin=\"nexus.example.com\",key=\"k\",sig=\"s\"";
        assert!(parse_auth_header(bad).is_err());
    }

    #[test]
    fn parse_auth_header_empty_errors() {
        assert!(parse_auth_header("").is_err());
    }
}
