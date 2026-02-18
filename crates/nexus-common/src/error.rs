//! Centralized error types for Nexus.
//!
//! Uses `thiserror` for ergonomic error definitions and provides HTTP-friendly
//! error variants that can be directly converted to API responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Core application error type used across all Nexus services.
#[derive(Debug, thiserror::Error)]
pub enum NexusError {
    // === Auth errors ===
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Unauthorized")]
    Unauthorized,

    // === Resource errors ===
    #[error("{resource} not found")]
    NotFound { resource: String },

    #[error("{resource} already exists")]
    AlreadyExists { resource: String },

    // === Validation errors ===
    #[error("Validation failed: {message}")]
    Validation { message: String },

    // === Permission errors ===
    #[error("Missing permission: {permission}")]
    MissingPermission { permission: String },

    #[error("Forbidden")]
    Forbidden,

    // === Rate limiting ===
    #[error("Rate limited. Retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    // === Server capacity ===
    #[error("Limit reached: {message}")]
    LimitReached { message: String },

    // === Infrastructure errors ===
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// JSON error response body sent to clients.
#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    error: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_ms: Option<u64>,
}

impl NexusError {
    /// Map error to HTTP status code.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidCredentials | Self::InvalidToken => StatusCode::UNAUTHORIZED,
            Self::TokenExpired => StatusCode::UNAUTHORIZED,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::AlreadyExists { .. } => StatusCode::CONFLICT,
            Self::Validation { .. } => StatusCode::BAD_REQUEST,
            Self::MissingPermission { .. } | Self::Forbidden => StatusCode::FORBIDDEN,
            Self::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::LimitReached { .. } => StatusCode::FORBIDDEN,
            Self::Database(_) | Self::Redis(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    /// Error code string for programmatic handling by clients.
    pub fn error_code(&self) -> &str {
        match self {
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::TokenExpired => "TOKEN_EXPIRED",
            Self::InvalidToken => "INVALID_TOKEN",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::AlreadyExists { .. } => "ALREADY_EXISTS",
            Self::Validation { .. } => "VALIDATION_ERROR",
            Self::MissingPermission { .. } => "MISSING_PERMISSION",
            Self::Forbidden => "FORBIDDEN",
            Self::RateLimited { .. } => "RATE_LIMITED",
            Self::LimitReached { .. } => "LIMIT_REACHED",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Redis(_) => "CACHE_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

impl IntoResponse for NexusError {
    fn into_response(self) -> Response {
        let status = self.status_code();

        // Don't leak internal details to clients
        let message = match &self {
            NexusError::Database(e) => {
                tracing::error!("Database error: {e}");
                "An internal error occurred".to_string()
            }
            NexusError::Redis(e) => {
                tracing::error!("Redis error: {e}");
                "An internal error occurred".to_string()
            }
            NexusError::Internal(e) => {
                tracing::error!("Internal error: {e}");
                "An internal error occurred".to_string()
            }
            other => other.to_string(),
        };

        let retry_after_ms = if let NexusError::RateLimited { retry_after_ms } = &self {
            Some(*retry_after_ms)
        } else {
            None
        };

        let body = ErrorResponse {
            code: status.as_u16(),
            error: self.error_code().to_string(),
            message,
            retry_after_ms,
        };

        (status, axum::Json(body)).into_response()
    }
}

/// Convenience type alias for Results using NexusError.
pub type NexusResult<T> = Result<T, NexusError>;
