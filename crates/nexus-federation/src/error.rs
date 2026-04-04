//! Federation-specific error types.

use thiserror::Error;

/// Errors that can occur in federation operations.
#[derive(Debug, Error)]
pub enum FederationError {
    // ── Key management ──────────────────────────────────────────────────────

    #[error("No signing key found for key ID '{0}'")]
    KeyNotFound(String),

    #[error("Failed to generate signing key: {0}")]
    KeyGeneration(String),

    #[error("Failed to load signing key from storage: {0}")]
    KeyLoad(String),

    // ── Signature verification ───────────────────────────────────────────────

    #[error("Missing Authorization header on federated request")]
    MissingAuthHeader,

    #[error("Malformed Authorization header: {0}")]
    MalformedAuthHeader(String),

    #[error("Signature verification failed")]
    InvalidSignature,

    #[error("Request timestamp too skewed (max ±30s)")]
    ClockSkew,

    // ── Discovery ───────────────────────────────────────────────────────────

    #[error("Failed to resolve server '{0}': {1}")]
    DiscoveryFailed(String, String),

    #[error("Server '{0}' returned a bad well-known response")]
    BadWellKnown(String),

    // ── Remote communication ─────────────────────────────────────────────────

    #[error("HTTP error communicating with remote server '{0}': {1}")]
    RemoteHttp(String, String),

    #[error("Remote server '{0}' returned an unexpected response: {1}")]
    RemoteProtocol(String, String),

    #[error("Remote server '{0}' is not reachable")]
    RemoteUnreachable(String),

    // ── General ─────────────────────────────────────────────────────────────

    #[error("Serialisation error: {0}")]
    Serialisation(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<reqwest::Error> for FederationError {
    fn from(e: reqwest::Error) -> Self {
        let server = e.url().map(|u| u.host_str().unwrap_or("?").to_owned()).unwrap_or_default();
        FederationError::RemoteHttp(server, e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── FederationError display strings ──────────────────────────────────────

    #[test]
    fn key_not_found_includes_key_id() {
        let e = FederationError::KeyNotFound("ed25519:abc123".to_string());
        assert!(e.to_string().contains("ed25519:abc123"));
    }

    #[test]
    fn malformed_auth_header_includes_reason() {
        let e = FederationError::MalformedAuthHeader("missing sig".to_string());
        assert!(e.to_string().contains("missing sig"));
    }

    #[test]
    fn discovery_failed_includes_server_and_reason() {
        let e = FederationError::DiscoveryFailed(
            "nexus.example.com".to_string(),
            "connection refused".to_string(),
        );
        let s = e.to_string();
        assert!(s.contains("nexus.example.com"));
        assert!(s.contains("connection refused"));
    }

    #[test]
    fn remote_http_includes_server_and_message() {
        let e = FederationError::RemoteHttp(
            "matrix.org".to_string(),
            "503 Service Unavailable".to_string(),
        );
        let s = e.to_string();
        assert!(s.contains("matrix.org"));
        assert!(s.contains("503"));
    }

    #[test]
    fn clock_skew_error_has_descriptive_message() {
        let s = FederationError::ClockSkew.to_string();
        assert!(s.contains("30") || s.contains("skew") || s.contains("timestamp"),
            "expected clock-skew description, got: {s}");
    }

    #[test]
    fn missing_auth_header_has_non_empty_message() {
        assert!(!FederationError::MissingAuthHeader.to_string().is_empty());
    }

    #[test]
    fn invalid_signature_has_non_empty_message() {
        assert!(!FederationError::InvalidSignature.to_string().is_empty());
    }

    #[test]
    fn from_serde_json_error_converts_correctly() {
        let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let fed_err: FederationError = json_err.into();
        assert!(matches!(fed_err, FederationError::Serialisation(_)));
    }

    #[test]
    fn from_url_parse_error_converts_correctly() {
        let url_err = "not a url !!!".parse::<url::Url>().unwrap_err();
        let fed_err: FederationError = url_err.into();
        assert!(matches!(fed_err, FederationError::UrlParse(_)));
    }
}
