//! Error types for the Nexus SDK.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NexusError {
    /// The HTTP response had a non-2xx status code.
    #[error("API error {status}: {message}")]
    Api { status: u16, message: String },

    /// An error from the underlying HTTP client.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// An error from the WebSocket layer.
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// A JSON (de)serialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The gateway was not connected.
    #[error("Gateway is not connected")]
    NotConnected,

    /// A generic error string.
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, NexusError>;
