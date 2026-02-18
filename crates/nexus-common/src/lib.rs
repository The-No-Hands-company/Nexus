//! # nexus-common
//!
//! Shared types, configuration, error handling, and utilities used across all Nexus crates.
//! This is the foundation layer — no business logic, just primitives and contracts.

pub mod config;
pub mod error;
pub mod models;
pub mod permissions;
pub mod snowflake;
pub mod validation;
