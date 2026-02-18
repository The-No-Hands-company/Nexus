//! Input validation utilities.
//!
//! Centralized validation helpers used across API routes.

use validator::Validate;

use crate::error::NexusError;

/// Validate a request body, returning a NexusError::Validation on failure.
pub fn validate_request<T: Validate>(body: &T) -> Result<(), NexusError> {
    body.validate().map_err(|e| NexusError::Validation {
        message: format_validation_errors(e),
    })
}

/// Format validation errors into a human-readable string.
fn format_validation_errors(errors: validator::ValidationErrors) -> String {
    errors
        .field_errors()
        .iter()
        .flat_map(|(field, errs)| {
            errs.iter().map(move |e| {
                let msg = e
                    .message
                    .as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| format!("Invalid value for '{field}'"));
                msg
            })
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Validate that a string is a safe channel/server name (no special chars that break routing).
pub fn validate_name(name: &str) -> Result<(), NexusError> {
    if name.trim().is_empty() {
        return Err(NexusError::Validation {
            message: "Name cannot be empty or whitespace only".into(),
        });
    }

    // Channel names: lowercase, alphanumeric, hyphens, underscores
    let valid = name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ');

    if !valid {
        return Err(NexusError::Validation {
            message: "Name can only contain letters, numbers, hyphens, underscores, and spaces"
                .into(),
        });
    }

    Ok(())
}
