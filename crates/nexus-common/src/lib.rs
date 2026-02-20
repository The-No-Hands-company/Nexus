//! # nexus-common
//!
//! Shared types, configuration, error handling, and utilities used across all Nexus crates.
//! This is the foundation layer — no business logic, just primitives and contracts.

pub mod auth;
pub mod config;
pub mod crypto;
pub mod error;
pub mod gateway_event;
pub mod models;
pub mod permissions;
pub mod snowflake;
pub mod validation;
/// Manual `sqlx::FromRow<'_, AnyRow>` impls for all model types (AnyPool compat).
pub mod any_row;
