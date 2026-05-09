//! Input validation utilities.
//!
//! Centralized validation helpers used across API routes.

use validator::Validate;

use crate::error::NexusError;

/// Validate a request body, returning a `NexusError::Validation` on failure.
///
/// # Errors
/// Returns `NexusError::Validation` when the input fails validator checks.
pub fn validate_request<T: Validate>(body: &T) -> Result<(), NexusError> {
    body.validate().map_err(|e| NexusError::Validation {
        message: format_validation_errors(&e),
    })
}

/// Format validation errors into a human-readable string.
fn format_validation_errors(errors: &validator::ValidationErrors) -> String {
    errors
        .field_errors()
        .iter()
        .flat_map(|(field, errs)| {
            errs.iter().map(move |e| {
                e.message.as_ref().map_or_else(
                    || format!("Invalid value for '{field}'"),
                    std::string::ToString::to_string,
                )
            })
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Validate that a string is a safe channel/server name (no special chars that break routing).
///
/// # Errors
/// Returns `NexusError::Validation` when the provided name is empty/whitespace
/// or contains unsupported characters.
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

/// Extract user UUIDs mentioned via the `<@uuid>` syntax from message content.
///
/// Returns a deduplicated `Vec<Uuid>` — used by message creation, webhook
/// execution, and any other path that stores and broadcasts mention data.
#[must_use]
pub fn parse_mentions_from_content(content: &str) -> Vec<uuid::Uuid> {
    let mut mentions: Vec<uuid::Uuid> = Vec::new();
    for token in content.split_whitespace() {
        if let Some(id_str) = token.strip_prefix("<@").and_then(|s| s.strip_suffix('>'))
            && let Ok(id) = id_str.parse::<uuid::Uuid>()
            && !mentions.contains(&id)
        {
            mentions.push(id);
        }
    }
    mentions
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_name ────────────────────────────────────────────────────────

    #[test]
    fn empty_string_is_rejected() {
        assert!(validate_name("").is_err());
    }

    #[test]
    fn whitespace_only_is_rejected() {
        assert!(validate_name("   ").is_err());
        assert!(validate_name("\t").is_err());
    }

    #[test]
    fn valid_simple_names_are_accepted() {
        assert!(validate_name("general").is_ok());
        assert!(validate_name("General").is_ok());
        assert!(validate_name("my-channel").is_ok());
        assert!(validate_name("my_channel").is_ok());
        assert!(validate_name("channel 42").is_ok());
        assert!(validate_name("CoolServer2026").is_ok());
    }

    #[test]
    fn special_chars_are_rejected() {
        assert!(validate_name("hello!").is_err());
        assert!(validate_name("chan#name").is_err());
        assert!(validate_name("server@home").is_err());
        assert!(validate_name("path/traversal").is_err());
        assert!(validate_name("semi;colon").is_err());
    }

    #[test]
    fn error_variant_is_validation() {
        let err = validate_name("!bad!").unwrap_err();
        assert!(matches!(err, crate::error::NexusError::Validation { .. }));
    }

    #[test]
    fn unicode_letters_are_accepted() {
        assert!(validate_name("Ñexus").is_ok());
        assert!(validate_name("Ñèxùs").is_ok());
    }

    // ── validate_request ─────────────────────────────────────────────────────

    #[derive(Debug, validator::Validate)]
    struct Dummy {
        #[validate(length(min = 1, max = 10, message = "too long or empty"))]
        pub name: String,
    }

    #[test]
    fn validate_request_passes_for_valid_data() {
        let req = Dummy {
            name: "hello".into(),
        };
        assert!(validate_request(&req).is_ok());
    }

    #[test]
    fn validate_request_fails_and_formats_error() {
        let req = Dummy {
            name: "this is way too long for the limit".into(),
        };
        let err = validate_request(&req).unwrap_err();
        // Error should be a Validation variant with a non-empty message
        match err {
            crate::error::NexusError::Validation { message } => {
                assert!(!message.is_empty());
            }
            other => panic!("expected Validation error, got {other:?}"),
        }
    }
}
