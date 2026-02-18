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
